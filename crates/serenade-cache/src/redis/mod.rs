//! Optional Redis adapter (feature `redis`).

mod adapter;
mod config;
mod keys;

pub use adapter::RedisAdapter;
pub use config::RedisAdapterConfig;
pub use keys::redis_key;
