//! [`LockStore`] persistence contract.

use std::time::Duration;

use crate::LockError;

/// Low-level lock persistence (Symfony `PersistingStoreInterface` analogue).
pub trait LockStore: Send + Sync {
    /// Persists ownership for `resource` under `token`.
    ///
    /// When `ttl` is `Some`, ownership expires after that duration. `None` means no expiry.
    /// Same-token re-save refreshes ownership. Another token yields [`LockError::Conflict`].
    ///
    /// # Errors
    ///
    /// Returns [`LockError`] on invalid resource, conflict, or store failure.
    fn save(&self, resource: &str, token: &str, ttl: Option<Duration>) -> Result<(), LockError>;

    /// Drops ownership when `token` matches (idempotent when missing or expired).
    ///
    /// # Errors
    ///
    /// Returns [`LockError`] on store failure.
    fn delete(&self, resource: &str, token: &str) -> Result<(), LockError>;

    /// True when `token` currently owns `resource` and the entry has not expired.
    ///
    /// # Errors
    ///
    /// Returns [`LockError`] on store failure.
    fn exists(&self, resource: &str, token: &str) -> Result<bool, LockError>;

    /// Extends TTL for a held lock. `ttl: None` clears expiry (hold forever).
    ///
    /// # Errors
    ///
    /// Returns [`LockError::NotHeld`] when this token does not own the resource.
    fn put_off_expiration(
        &self,
        resource: &str,
        token: &str,
        ttl: Option<Duration>,
    ) -> Result<(), LockError>;
}
