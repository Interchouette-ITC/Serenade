//! DI tag and compile pass for the default rate-limiter storage.

use std::sync::Arc;
use std::time::Duration;

use serenade_di::{CompilePass, ContainerBuilder, DiError, ServiceDefinition};

use crate::{
    InMemoryRateLimiterStorage, Policy, RateLimiterError, RateLimiterFactory, RateLimiterStorage,
};

/// Service tag applied to rate-limiter storage definitions.
pub const RATE_LIMITER_STORAGE_TAG: &str = "rate_limiter.storage";

/// Container id of the default rate-limiter storage when registered by the pass.
pub const DEFAULT_RATE_LIMITER_STORAGE_SERVICE: &str = "rate_limiter.storage";

/// Newtype so [`RateLimiterStorage`] trait objects can be stored in the container.
#[derive(Clone)]
pub struct RateLimiterStorageService(pub Arc<dyn RateLimiterStorage>);

impl RateLimiterStorageService {
    /// Builds a [`RateLimiterFactory`] against the wrapped storage.
    ///
    /// # Errors
    ///
    /// Propagates empty `name` or invalid `policy` construction errors from the caller-built policy.
    pub fn factory(
        &self,
        name: impl Into<String>,
        policy: Policy,
    ) -> Result<RateLimiterFactory, RateLimiterError> {
        RateLimiterFactory::new(name, policy, Arc::clone(&self.0))
    }

    /// Convenience factory: fixed window of `limit` tokens per `interval`.
    ///
    /// # Errors
    ///
    /// Propagates policy and factory validation errors.
    pub fn fixed_window_factory(
        &self,
        name: impl Into<String>,
        limit: u32,
        interval: Duration,
    ) -> Result<RateLimiterFactory, RateLimiterError> {
        self.factory(name, Policy::fixed_window(limit, interval)?)
    }

    /// Convenience factory: token bucket of `limit` tokens refill over `interval`.
    ///
    /// # Errors
    ///
    /// Propagates policy and factory validation errors.
    pub fn token_bucket_factory(
        &self,
        name: impl Into<String>,
        limit: u32,
        interval: Duration,
    ) -> Result<RateLimiterFactory, RateLimiterError> {
        self.factory(name, Policy::token_bucket(limit, interval)?)
    }
}

/// Seeds [`DEFAULT_RATE_LIMITER_STORAGE_SERVICE`] with an [`InMemoryRateLimiterStorage`] when missing.
#[derive(Debug, Default)]
pub struct RegisterDefaultRateLimiterPass;

impl CompilePass for RegisterDefaultRateLimiterPass {
    fn name(&self) -> &'static str {
        "register_default_rate_limiter_storage"
    }

    fn process(&self, builder: &mut ContainerBuilder) -> Result<(), DiError> {
        let has_default = builder
            .definitions()
            .iter()
            .any(|definition| definition.id() == DEFAULT_RATE_LIMITER_STORAGE_SERVICE);
        if has_default {
            return Ok(());
        }

        // `expect`: default id cannot collide after the has_default guard above.
        builder
            .register(
                ServiceDefinition::new(DEFAULT_RATE_LIMITER_STORAGE_SERVICE)
                    .with_tag(RATE_LIMITER_STORAGE_TAG),
                |_container| {
                    Ok(Box::new(RateLimiterStorageService(
                        Arc::new(InMemoryRateLimiterStorage::new()) as Arc<dyn RateLimiterStorage>,
                    )))
                },
            )
            .expect("default rate_limiter.storage id is unique");

        Ok(())
    }
}
