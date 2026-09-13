//! [`LockFactory`] builds [`Lock`] handles.

use std::sync::Arc;
use std::time::Duration;

use crate::{Lock, LockError, LockStore, generate_lock_token, validate_resource};

/// Default TTL when [`LockFactory::create_lock`] receives `ttl: None` (Symfony default).
pub const DEFAULT_LOCK_TTL: Duration = Duration::from_secs(300);

/// Creates [`Lock`] handles against a shared [`LockStore`].
#[derive(Clone)]
pub struct LockFactory {
    store: Arc<dyn LockStore>,
}

impl LockFactory {
    /// Wraps `store`.
    #[must_use]
    pub fn new(store: Arc<dyn LockStore>) -> Self {
        Self { store }
    }

    /// Builds a lock for `resource` without acquiring it.
    ///
    /// When `ttl` is `None`, uses [`DEFAULT_LOCK_TTL`]. For no expiry, call
    /// [`Self::create_lock_forever`].
    ///
    /// # Errors
    ///
    /// Returns [`LockError::InvalidKey`] or [`LockError::Generation`].
    pub fn create_lock(
        &self,
        resource: impl Into<String>,
        ttl: Option<Duration>,
        auto_release: bool,
    ) -> Result<Lock, LockError> {
        let ttl = Some(ttl.unwrap_or(DEFAULT_LOCK_TTL));
        self.build(resource, ttl, auto_release)
    }

    /// Builds a lock that does not expire until released.
    ///
    /// # Errors
    ///
    /// Returns [`LockError::InvalidKey`] or [`LockError::Generation`].
    pub fn create_lock_forever(
        &self,
        resource: impl Into<String>,
        auto_release: bool,
    ) -> Result<Lock, LockError> {
        self.build(resource, None, auto_release)
    }

    fn build(
        &self,
        resource: impl Into<String>,
        ttl: Option<Duration>,
        auto_release: bool,
    ) -> Result<Lock, LockError> {
        let resource = resource.into();
        validate_resource(&resource)?;
        let token = generate_lock_token()?;
        Ok(Lock::new(
            resource,
            token,
            Arc::clone(&self.store),
            ttl,
            auto_release,
        ))
    }
}
