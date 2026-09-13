//! App-facing consume helpers.

use crate::{RateLimit, RateLimiter, RateLimiterError};

/// Rejected consume that still carries the [`RateLimit`] snapshot.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "rate limit exceeded (limit={limit}, remaining={remaining_tokens})",
    limit = self.rate_limit.limit(),
    remaining_tokens = self.rate_limit.remaining_tokens()
)]
pub struct RateLimitExceeded {
    /// Decision returned by the limiter.
    pub rate_limit: RateLimit,
}

impl RateLimitExceeded {
    /// Suggested wait before retrying.
    #[must_use]
    pub const fn retry_after(&self) -> Option<std::time::Duration> {
        self.rate_limit.retry_after()
    }
}

/// Returns `Ok(rate_limit)` when accepted, otherwise [`RateLimitExceeded`].
///
/// # Errors
///
/// Returns [`RateLimitExceeded`] when `rate_limit` was rejected.
pub const fn require_accepted(rate_limit: RateLimit) -> Result<RateLimit, RateLimitExceeded> {
    if rate_limit.is_accepted() {
        Ok(rate_limit)
    } else {
        Err(RateLimitExceeded { rate_limit })
    }
}

/// Consumes `tokens` and requires acceptance.
///
/// # Errors
///
/// Propagates storage / validation errors, or [`RateLimitExceeded`] when rejected.
pub fn consume_or_exceed(
    limiter: &RateLimiter,
    tokens: u32,
) -> Result<RateLimit, ConsumeOrExceedError> {
    let rate_limit = limiter.consume(tokens)?;
    require_accepted(rate_limit).map_err(ConsumeOrExceedError::Exceeded)
}

/// Error from [`consume_or_exceed`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConsumeOrExceedError {
    /// Underlying limiter failure.
    #[error(transparent)]
    Limiter(#[from] RateLimiterError),
    /// Policy rejected the consume.
    #[error(transparent)]
    Exceeded(#[from] RateLimitExceeded),
}
