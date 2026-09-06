//! PSR-like cache contracts and an in-memory [`ArrayAdapter`].
//!
//! Redis adapters are deferred. DI tag: [`CACHE_POOL_TAG`].

mod compile_pass;
mod error;
mod item;
mod pool;

pub use compile_pass::{
    CachePoolService, RegisterDefaultCachePoolPass, CACHE_POOL_TAG, DEFAULT_CACHE_POOL_SERVICE,
};
pub use error::CacheError;
pub use item::{ArrayCacheItem, CacheItem};
pub use pool::{ArrayAdapter, CacheItemPool};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
