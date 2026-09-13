//! Symfony-shaped named locks.
//!
//! - [`LockStore`]: persist ownership for a resource + token
//! - [`LockFactory`] / [`Lock`]: acquire, release, refresh, `is_acquired`
//! - [`InMemoryLockStore`]: process-local store with optional TTL

mod error;
mod factory;
mod key;
mod lock;
mod memory;
mod store;

pub use error::LockError;
pub use factory::{DEFAULT_LOCK_TTL, LockFactory};
pub use key::{LockKey, generate_lock_token, validate_resource};
pub use lock::Lock;
pub use memory::InMemoryLockStore;
pub use store::LockStore;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
