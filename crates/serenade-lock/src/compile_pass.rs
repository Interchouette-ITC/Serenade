//! DI tag and compile pass for the default lock store.

use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, DiError, ServiceDefinition};

use crate::{InMemoryLockStore, LockFactory, LockStore};

/// Service tag applied to lock store definitions.
pub const LOCK_STORE_TAG: &str = "lock.store";

/// Container id of the default lock store when registered by the pass.
pub const DEFAULT_LOCK_STORE_SERVICE: &str = "lock.store";

/// Newtype so [`LockStore`] trait objects can be stored in the container.
#[derive(Clone)]
pub struct LockStoreService(pub Arc<dyn LockStore>);

impl LockStoreService {
    /// Builds a [`LockFactory`] against the wrapped store.
    #[must_use]
    pub fn factory(&self) -> LockFactory {
        LockFactory::new(Arc::clone(&self.0))
    }
}

/// Seeds [`DEFAULT_LOCK_STORE_SERVICE`] with an [`InMemoryLockStore`] when missing.
#[derive(Debug, Default)]
pub struct RegisterDefaultLockPass;

impl CompilePass for RegisterDefaultLockPass {
    fn name(&self) -> &'static str {
        "register_default_lock_store"
    }

    fn process(&self, builder: &mut ContainerBuilder) -> Result<(), DiError> {
        let has_default = builder
            .definitions()
            .iter()
            .any(|definition| definition.id() == DEFAULT_LOCK_STORE_SERVICE);
        if has_default {
            return Ok(());
        }

        // `expect`: default id cannot collide after the has_default guard above.
        builder
            .register(
                ServiceDefinition::new(DEFAULT_LOCK_STORE_SERVICE).with_tag(LOCK_STORE_TAG),
                |_container| {
                    Ok(Box::new(LockStoreService(
                        Arc::new(InMemoryLockStore::new()) as Arc<dyn LockStore>,
                    )))
                },
            )
            .expect("default lock.store id is unique");

        Ok(())
    }
}
