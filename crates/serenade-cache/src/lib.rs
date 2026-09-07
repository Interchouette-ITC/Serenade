//! PSR-like cache contracts and an in-memory [`ArrayAdapter`].
//!
//! Enable Cargo feature `redis` for `RedisAdapter` (redis-rs + r2d2).

mod compile_pass;
mod error;
mod item;
mod pool;

#[cfg(feature = "redis")]
mod redis;

pub use compile_pass::{
    CACHE_POOL_TAG, CachePoolService, DEFAULT_CACHE_POOL_SERVICE, RegisterDefaultCachePoolPass,
};
pub use error::CacheError;
pub use item::{ArrayCacheItem, CacheItem};
pub use pool::{ArrayAdapter, CacheItemPool};

#[cfg(feature = "redis")]
pub use redis::{
    BytesMarshaller, CacheMarshaller, RedisAdapter, RedisAdapterConfig, redis_key,
    validate_logical_key,
};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
