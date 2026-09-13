//! User-facing [`Lock`] handle.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{LockError, LockStore};

/// Named lock bound to one owner token and a [`LockStore`].
pub struct Lock {
    resource: String,
    token: String,
    store: Arc<dyn LockStore>,
    ttl: Option<Duration>,
    auto_release: bool,
    acquired: Mutex<bool>,
}

impl Lock {
    pub(crate) fn new(
        resource: String,
        token: String,
        store: Arc<dyn LockStore>,
        ttl: Option<Duration>,
        auto_release: bool,
    ) -> Self {
        Self {
            resource,
            token,
            store,
            ttl,
            auto_release,
            acquired: Mutex::new(false),
        }
    }

    /// Resource name.
    #[must_use]
    pub fn resource(&self) -> &str {
        &self.resource
    }

    /// Owner token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Default TTL used on acquire / refresh when callers pass `None`.
    #[must_use]
    pub const fn ttl(&self) -> Option<Duration> {
        self.ttl
    }

    /// Tries to acquire without blocking.
    ///
    /// Returns `Ok(true)` when acquired (or already held by this token), `Ok(false)` on conflict.
    ///
    /// # Errors
    ///
    /// Propagates store failures other than conflict.
    ///
    /// # Panics
    ///
    /// Panics when the internal acquired flag mutex is poisoned.
    pub fn acquire(&self) -> Result<bool, LockError> {
        match self.store.save(&self.resource, &self.token, self.ttl) {
            Ok(()) => {
                *self
                    .acquired
                    .lock()
                    .expect("serenade-lock acquired flag poisoned") = true;
                Ok(true)
            }
            Err(LockError::Conflict { .. }) => Ok(false),
            Err(err) => Err(err),
        }
    }

    /// Releases ownership when held by this token.
    ///
    /// # Errors
    ///
    /// Propagates store failures.
    ///
    /// # Panics
    ///
    /// Panics when the internal acquired flag mutex is poisoned.
    pub fn release(&self) -> Result<(), LockError> {
        self.store.delete(&self.resource, &self.token)?;
        *self
            .acquired
            .lock()
            .expect("serenade-lock acquired flag poisoned") = false;
        Ok(())
    }

    /// Extends TTL. `ttl: None` uses the lock default TTL (which may also be `None` = forever).
    ///
    /// # Errors
    ///
    /// Returns [`LockError::NotHeld`] when this token does not own the resource.
    ///
    /// # Panics
    ///
    /// Panics when the internal acquired flag mutex is poisoned.
    pub fn refresh(&self, ttl: Option<Duration>) -> Result<(), LockError> {
        let ttl = ttl.or(self.ttl);
        self.store
            .put_off_expiration(&self.resource, &self.token, ttl)?;
        *self
            .acquired
            .lock()
            .expect("serenade-lock acquired flag poisoned") = true;
        Ok(())
    }

    /// Store-backed ownership check.
    ///
    /// # Errors
    ///
    /// Propagates store failures.
    pub fn is_acquired(&self) -> Result<bool, LockError> {
        self.store.exists(&self.resource, &self.token)
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        if !self.auto_release {
            return;
        }
        let mut acquired = self
            .acquired
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !*acquired {
            return;
        }
        let _ = self.store.delete(&self.resource, &self.token);
        *acquired = false;
    }
}
