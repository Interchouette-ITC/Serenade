//! [`RateLimiterFactory`].

use std::sync::Arc;

use crate::{Policy, RateLimiter, RateLimiterError, RateLimiterStorage, validate_key};

/// Builds [`RateLimiter`] handles that share one policy and storage.
#[derive(Clone)]
pub struct RateLimiterFactory {
    name: String,
    policy: Policy,
    storage: Arc<dyn RateLimiterStorage>,
}

impl RateLimiterFactory {
    /// Factory for limiter `name` using `policy` and `storage`.
    ///
    /// # Errors
    ///
    /// Returns [`RateLimiterError::InvalidKey`] when `name` is empty.
    pub fn new(
        name: impl Into<String>,
        policy: Policy,
        storage: Arc<dyn RateLimiterStorage>,
    ) -> Result<Self, RateLimiterError> {
        let name = name.into();
        validate_key(&name)?;
        Ok(Self {
            name,
            policy,
            storage,
        })
    }

    /// Limiter name prefix.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Shared policy.
    #[must_use]
    pub const fn policy(&self) -> &Policy {
        &self.policy
    }

    /// Creates a limiter for subject `key`.
    ///
    /// # Errors
    ///
    /// Returns [`RateLimiterError::InvalidKey`] when `key` is empty.
    pub fn create(&self, key: &str) -> Result<RateLimiter, RateLimiterError> {
        validate_key(key)?;
        let id = format!("{}-{key}", self.name);
        Ok(RateLimiter::new(
            id,
            self.policy.clone(),
            Arc::clone(&self.storage),
        ))
    }
}
