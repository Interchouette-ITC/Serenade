//! Symfony-shaped rate limiting.
//!
//! - [`Policy`]: token-bucket or fixed-window limits
//! - [`RateLimiter`] / [`RateLimiterFactory`]: `consume` / `reset`
//! - [`RateLimiterStorage`] / [`InMemoryRateLimiterStorage`]: persist window state
//! - [`RateLimit`]: accept / remaining / retry-after

mod error;
mod factory;
mod limiter;
mod memory;
mod policy;
mod rate_limit;
mod storage;

pub use error::RateLimiterError;
pub use factory::RateLimiterFactory;
pub use limiter::RateLimiter;
pub use memory::InMemoryRateLimiterStorage;
pub use policy::Policy;
pub use rate_limit::RateLimit;
pub use storage::{RateLimiterStorage, WindowState};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Rejects empty limiter keys.
///
/// # Errors
///
/// Returns [`RateLimiterError::InvalidKey`] when `key` is empty.
pub fn validate_key(key: &str) -> Result<(), RateLimiterError> {
    if key.is_empty() {
        return Err(RateLimiterError::InvalidKey {
            key: key.to_owned(),
            message: "key must not be empty".to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
