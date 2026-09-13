//! Outcome of a [`crate::RateLimiter::consume`] call.

use std::time::Duration;

/// Accept / reject decision with remaining budget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RateLimit {
    accepted: bool,
    limit: u32,
    remaining_tokens: u32,
    retry_after: Option<Duration>,
}

impl RateLimit {
    pub(crate) const fn accepted(
        limit: u32,
        remaining_tokens: u32,
        retry_after: Option<Duration>,
    ) -> Self {
        Self {
            accepted: true,
            limit,
            remaining_tokens,
            retry_after,
        }
    }

    pub(crate) const fn rejected(
        limit: u32,
        remaining_tokens: u32,
        retry_after: Option<Duration>,
    ) -> Self {
        Self {
            accepted: false,
            limit,
            remaining_tokens,
            retry_after,
        }
    }

    /// Whether the consume was accepted.
    #[must_use]
    pub const fn is_accepted(&self) -> bool {
        self.accepted
    }

    /// Configured limit for this policy.
    #[must_use]
    pub const fn limit(&self) -> u32 {
        self.limit
    }

    /// Tokens remaining after this decision.
    #[must_use]
    pub const fn remaining_tokens(&self) -> u32 {
        self.remaining_tokens
    }

    /// Suggested wait before retrying when rejected.
    #[must_use]
    pub const fn retry_after(&self) -> Option<Duration> {
        self.retry_after
    }
}
