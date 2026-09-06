//! DI tag and compile pass for cache pools.

use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, DiError, ServiceDefinition};

use crate::{ArrayAdapter, CacheItemPool};

/// Service tag applied to cache pool definitions.
pub const CACHE_POOL_TAG: &str = "cache.pool";

/// Container id of the default compiled cache pool when registered by the pass.
pub const DEFAULT_CACHE_POOL_SERVICE: &str = "cache.app";

/// Newtype so pool trait objects can be stored in the container.
#[derive(Clone)]
pub struct CachePoolService(pub Arc<dyn CacheItemPool>);

/// Seeds [`DEFAULT_CACHE_POOL_SERVICE`] with an [`ArrayAdapter`] when missing.
///
/// Apps that need another pool register it under [`CACHE_POOL_TAG`] themselves
/// (and may alias `cache.app` via DI).
#[derive(Debug, Default)]
pub struct RegisterDefaultCachePoolPass;

impl CompilePass for RegisterDefaultCachePoolPass {
    fn name(&self) -> &'static str {
        "register_default_cache_pool"
    }

    fn process(&self, builder: &mut ContainerBuilder) -> Result<(), DiError> {
        let has_default = builder
            .definitions()
            .iter()
            .any(|definition| definition.id() == DEFAULT_CACHE_POOL_SERVICE);
        if has_default {
            return Ok(());
        }

        builder.register(
            ServiceDefinition::new(DEFAULT_CACHE_POOL_SERVICE).with_tag(CACHE_POOL_TAG),
            |_container| {
                Ok(Box::new(CachePoolService(
                    Arc::new(ArrayAdapter::new()) as Arc<dyn CacheItemPool>
                )))
            },
        )
    }
}
