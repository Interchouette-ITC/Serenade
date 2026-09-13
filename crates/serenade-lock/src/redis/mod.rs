//! Redis-backed lock store (feature `redis`).

mod adapter;
mod config;

pub use adapter::RedisLockStore;
pub use config::RedisLockStoreConfig;
