//! Redis-backed rate-limiter storage (feature `redis`).

mod adapter;
mod config;
mod wire;

pub use adapter::RedisRateLimiterStorage;
pub use config::RedisRateLimiterStorageConfig;
