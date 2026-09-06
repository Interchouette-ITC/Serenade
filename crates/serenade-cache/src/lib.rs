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
    CachePoolService, RegisterDefaultCachePoolPass, CACHE_POOL_TAG, DEFAULT_CACHE_POOL_SERVICE,
};
pub use error::CacheError;
pub use item::{ArrayCacheItem, CacheItem};
pub use pool::{ArrayAdapter, CacheItemPool};

#[cfg(feature = "redis")]
pub use redis::{
    redis_key, validate_logical_key, BytesMarshaller, CacheMarshaller, RedisAdapter,
    RedisAdapterConfig,
};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
