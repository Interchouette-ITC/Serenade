//! Symfony-shaped named locks.
//!
//! - [`LockStore`]: persist ownership for a resource + token
//! - [`LockFactory`] / [`Lock`]: acquire, release, refresh, `is_acquired`
//! - [`InMemoryLockStore`]: process-local store with optional TTL
//! - [`FilesystemLockStore`]: disk-backed store
//! - [`RedisLockStore`] (feature `redis`): Redis-backed store
//! - [`RegisterDefaultLockPass`]: DI default service id [`DEFAULT_LOCK_STORE_SERVICE`]
//!
//! # Examples
//!
//! ```
//! use std::sync::Arc;
//! use std::time::Duration;
//! use serenade_lock::{InMemoryLockStore, LockFactory};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let store = Arc::new(InMemoryLockStore::new());
//! let factory = LockFactory::new(store);
//! let lock = factory.create_lock("cron:mail-blast", Some(Duration::from_secs(30)), true)?;
//! assert!(lock.acquire()?);
//! lock.release()?;
//! # Ok(())
//! # }
//! ```

mod compile_pass;
mod error;
mod factory;
mod filesystem;
mod key;
mod lock;
mod memory;
mod store;

#[cfg(feature = "redis")]
mod redis;

pub use compile_pass::{
    DEFAULT_LOCK_STORE_SERVICE, LOCK_STORE_TAG, LockStoreService, RegisterDefaultLockPass,
};
pub use error::LockError;
pub use factory::{DEFAULT_LOCK_TTL, LockFactory};
pub use filesystem::{FilesystemLockStore, FilesystemLockStoreConfig};
pub use key::{LockKey, generate_lock_token, validate_resource};
pub use lock::Lock;
pub use memory::InMemoryLockStore;
pub use store::LockStore;

#[cfg(feature = "redis")]
pub use redis::{RedisLockStore, RedisLockStoreConfig};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
