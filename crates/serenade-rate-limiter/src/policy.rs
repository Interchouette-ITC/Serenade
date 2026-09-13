//! Limiter policies.

use std::time::{Duration, Instant};

use crate::{RateLimit, RateLimiterError, WindowState};

/// How tokens are granted over time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Refills continuously up to `limit` over each `interval`.
    TokenBucket {
        /// Maximum tokens (burst size).
        limit: u32,
        /// Duration to fully refill an empty bucket.
        interval: Duration,
    },
    /// Resets the counter to zero at the end of each `interval`.
    FixedWindow {
        /// Maximum tokens per window.
        limit: u32,
        /// Window length.
        interval: Duration,
    },
}

impl Policy {
    /// Token-bucket policy.
    ///
    /// # Errors
    ///
    /// Returns [`RateLimiterError::InvalidTokens`] when `limit` is zero or `interval` is zero.
    pub fn token_bucket(limit: u32, interval: Duration) -> Result<Self, RateLimiterError> {
        validate_policy(limit, interval)?;
        Ok(Self::TokenBucket { limit, interval })
    }

    /// Fixed-window policy.
    ///
    /// # Errors
    ///
    /// Returns [`RateLimiterError::InvalidTokens`] when `limit` is zero or `interval` is zero.
    pub fn fixed_window(limit: u32, interval: Duration) -> Result<Self, RateLimiterError> {
        validate_policy(limit, interval)?;
        Ok(Self::FixedWindow { limit, interval })
    }

    /// Configured limit (burst / window max).
    #[must_use]
    pub const fn limit(&self) -> u32 {
        match *self {
            Self::TokenBucket { limit, .. } | Self::FixedWindow { limit, .. } => limit,
        }
    }

    /// Configured interval.
    #[must_use]
    pub const fn interval(&self) -> Duration {
        match *self {
            Self::TokenBucket { interval, .. } | Self::FixedWindow { interval, .. } => interval,
        }
    }

    /// Suggested storage TTL for this policy (2× interval).
    #[must_use]
    pub const fn storage_ttl(&self) -> Duration {
        self.interval().saturating_mul(2)
    }

    pub(crate) fn consume(
        &self,
        state: &mut Option<WindowState>,
        tokens: u32,
        now: Instant,
    ) -> Result<RateLimit, RateLimiterError> {
        if tokens == 0 {
            return Err(RateLimiterError::InvalidTokens {
                tokens: 0,
                message: "consume requires at least one token".to_owned(),
            });
        }
        match *self {
            Self::TokenBucket { limit, interval } => {
                Ok(consume_token_bucket(state, limit, interval, tokens, now))
            }
            Self::FixedWindow { limit, interval } => {
                consume_fixed_window(state, limit, interval, tokens, now)
            }
        }
    }
}

fn validate_policy(limit: u32, interval: Duration) -> Result<(), RateLimiterError> {
    if limit == 0 {
        return Err(RateLimiterError::InvalidTokens {
            tokens: 0,
            message: "policy limit must be greater than zero".to_owned(),
        });
    }
    if interval.is_zero() {
        return Err(RateLimiterError::InvalidTokens {
            tokens: limit,
            message: "policy interval must be greater than zero".to_owned(),
        });
    }
    Ok(())
}

fn consume_token_bucket(
    state: &mut Option<WindowState>,
    limit: u32,
    interval: Duration,
    tokens: u32,
    now: Instant,
) -> RateLimit {
    let limit_scaled = u128::from(limit) * SCALE;
    let (mut available, mut updated) = match state {
        Some(WindowState::TokenBucket {
            tokens_scaled,
            updated_at,
        }) => (*tokens_scaled, *updated_at),
        Some(WindowState::FixedWindow { .. }) | None => (limit_scaled, now),
    };

    if now > updated {
        let elapsed = now.duration_since(updated);
        let refill = refill_scaled(elapsed, limit_scaled, interval);
        available = (available + refill).min(limit_scaled);
        updated = now;
    }

    let need = u128::from(tokens) * SCALE;
    if available >= need {
        available -= need;
        *state = Some(WindowState::TokenBucket {
            tokens_scaled: available,
            updated_at: updated,
        });
        return RateLimit::accepted(limit, scaled_to_tokens(available), None);
    }

    let missing = need - available;
    let retry_after = retry_after_for_refill(missing, limit_scaled, interval);
    *state = Some(WindowState::TokenBucket {
        tokens_scaled: available,
        updated_at: updated,
    });
    RateLimit::rejected(limit, scaled_to_tokens(available), Some(retry_after))
}

fn consume_fixed_window(
    state: &mut Option<WindowState>,
    limit: u32,
    interval: Duration,
    tokens: u32,
    now: Instant,
) -> Result<RateLimit, RateLimiterError> {
    let (mut count, mut started) = match state {
        Some(WindowState::FixedWindow {
            count,
            window_start,
        }) => (*count, *window_start),
        Some(WindowState::TokenBucket { .. }) | None => (0, now),
    };

    if now.duration_since(started) >= interval {
        count = 0;
        started = now;
    }

    let Some(next) = count.checked_add(tokens) else {
        return Err(RateLimiterError::InvalidTokens {
            tokens,
            message: "token count overflow".to_owned(),
        });
    };

    if next <= limit {
        count = next;
        *state = Some(WindowState::FixedWindow {
            count,
            window_start: started,
        });
        return Ok(RateLimit::accepted(limit, limit - count, None));
    }

    let elapsed = now.duration_since(started);
    let retry_after = interval.saturating_sub(elapsed);
    *state = Some(WindowState::FixedWindow {
        count,
        window_start: started,
    });
    Ok(RateLimit::rejected(
        limit,
        limit.saturating_sub(count),
        Some(retry_after),
    ))
}

const SCALE: u128 = 1_000_000;

fn refill_scaled(elapsed: Duration, limit_scaled: u128, interval: Duration) -> u128 {
    let interval_ns = interval.as_nanos().max(1);
    elapsed.as_nanos().saturating_mul(limit_scaled) / interval_ns
}

fn scaled_to_tokens(tokens_scaled: u128) -> u32 {
    u32::try_from(tokens_scaled / SCALE).unwrap_or(u32::MAX)
}

fn retry_after_for_refill(
    missing_scaled: u128,
    limit_scaled: u128,
    interval: Duration,
) -> Duration {
    let interval_ns = interval.as_nanos();
    let wait_ns = missing_scaled
        .saturating_mul(interval_ns)
        .div_ceil(limit_scaled.max(1));
    Duration::from_nanos(u64::try_from(wait_ns).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod policy_tests {
    use super::Policy;
    use crate::WindowState;
    use std::time::{Duration, Instant};

    #[test]
    fn fixed_window_overflow_is_error() {
        let policy = Policy::fixed_window(1, Duration::from_secs(60)).expect("policy");
        let mut state = Some(WindowState::FixedWindow {
            count: u32::MAX,
            window_start: Instant::now(),
        });
        assert!(policy.consume(&mut state, 1, Instant::now()).is_err());
    }
}
