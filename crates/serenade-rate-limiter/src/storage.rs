//! Storage contract for limiter windows.

use std::time::{Duration, Instant};

use crate::{Policy, RateLimit, RateLimiterError};

/// Persisted window for one limiter subject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowState {
    /// Token-bucket snapshot (scaled integer tokens).
    TokenBucket {
        /// Available tokens multiplied by the internal scale factor.
        tokens_scaled: u128,
        /// Last refill timestamp.
        updated_at: Instant,
    },
    /// Fixed-window snapshot.
    FixedWindow {
        /// Tokens consumed in the current window.
        count: u32,
        /// Instant the current window started.
        window_start: Instant,
    },
}

/// Persists per-subject limiter windows.
pub trait RateLimiterStorage: Send + Sync {
    /// Atomically applies `policy.consume` for `id`.
    ///
    /// # Errors
    ///
    /// Propagates policy and storage failures.
    fn consume(
        &self,
        id: &str,
        policy: &Policy,
        tokens: u32,
        now: Instant,
        ttl: Option<Duration>,
    ) -> Result<RateLimit, RateLimiterError>;

    /// Deletes any stored window for `id`.
    ///
    /// # Errors
    ///
    /// Propagates storage failures.
    fn reset(&self, id: &str) -> Result<(), RateLimiterError>;
}
