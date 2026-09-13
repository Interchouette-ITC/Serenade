//! User-facing [`RateLimiter`] handle.

use std::sync::Arc;
use std::time::Instant;

use crate::{Policy, RateLimit, RateLimiterError, RateLimiterStorage};

/// Named limiter bound to one subject key and a [`RateLimiterStorage`].
#[derive(Clone)]
pub struct RateLimiter {
    id: String,
    policy: Policy,
    storage: Arc<dyn RateLimiterStorage>,
}

impl RateLimiter {
    pub(crate) fn new(id: String, policy: Policy, storage: Arc<dyn RateLimiterStorage>) -> Self {
        Self {
            id,
            policy,
            storage,
        }
    }

    /// Storage id (`{limiterName}-{key}`).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Policy used by this limiter.
    #[must_use]
    pub const fn policy(&self) -> &Policy {
        &self.policy
    }

    /// Consumes `tokens` from the budget.
    ///
    /// # Errors
    ///
    /// Propagates invalid token counts and storage failures.
    pub fn consume(&self, tokens: u32) -> Result<RateLimit, RateLimiterError> {
        self.storage.consume(
            &self.id,
            &self.policy,
            tokens,
            Instant::now(),
            Some(self.policy.storage_ttl()),
        )
    }

    /// Clears stored state for this subject.
    ///
    /// # Errors
    ///
    /// Propagates storage failures.
    pub fn reset(&self) -> Result<(), RateLimiterError> {
        self.storage.reset(&self.id)
    }
}
