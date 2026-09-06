//! DI tag and compile pass for cache pools.

use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, DiError, Reference, ServiceDefinition};

use crate::{ArrayAdapter, CacheItemPool};

/// Service tag applied to cache pool definitions.
pub const CACHE_POOL_TAG: &str = "cache.pool";

/// Container id of the default compiled cache pool when registered by the pass.
pub const DEFAULT_CACHE_POOL_SERVICE: &str = "cache.app";

/// Newtype so pool trait objects can be stored in the container.
#[derive(Clone)]
pub struct CachePoolService(pub Arc<dyn CacheItemPool>);

/// Ensures a default [`ArrayAdapter`] exists when no service is tagged [`CACHE_POOL_TAG`].
///
/// When tagged pools exist, registers aliases are left to the application; this pass only
/// seeds `cache.app` with an in-memory adapter if missing.
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

        let tagged: Vec<String> = builder
            .definitions()
            .iter()
            .filter(|definition| definition.tags().iter().any(|tag| tag == CACHE_POOL_TAG))
            .map(|definition| definition.id().to_owned())
            .collect();

        if let Some(first) = tagged.first() {
            let id = first.clone();
            builder.register(
                ServiceDefinition::new(DEFAULT_CACHE_POOL_SERVICE)
                    .with_dependencies(vec![Reference::from(id.clone())]),
                move |container| {
                    let pool = container.get_as::<CachePoolService>(&id)?;
                    Ok(Box::new(CachePoolService(Arc::clone(&pool.0))))
                },
            )?;
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
