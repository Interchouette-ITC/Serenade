//! PSR-like cache contracts, in-memory [`ArrayAdapter`], and [`FilesystemAdapter`].
//!
//! Enable Cargo feature `redis` for `RedisAdapter` (redis-rs + r2d2).

mod compile_pass;
mod error;
mod filesystem;
mod item;
mod key;
mod marshaller;
mod pool;

#[cfg(feature = "redis")]
mod redis;

pub use compile_pass::{
    CACHE_POOL_TAG, CachePoolService, DEFAULT_CACHE_POOL_SERVICE, RegisterDefaultCachePoolPass,
};
pub use error::CacheError;
pub use filesystem::{FilesystemAdapter, FilesystemAdapterConfig};
pub use item::{ArrayCacheItem, CacheItem};
pub use key::validate_logical_key;
pub use marshaller::{BytesMarshaller, CacheMarshaller};
pub use pool::{ArrayAdapter, CacheItemPool};

#[cfg(feature = "redis")]
pub use redis::{RedisAdapter, RedisAdapterConfig, redis_key};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
