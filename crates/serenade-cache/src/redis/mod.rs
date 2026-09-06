//! Optional Redis adapter (feature `redis`).

mod adapter;
mod config;
mod keys;
mod marshaller;

pub use adapter::RedisAdapter;
pub use config::RedisAdapterConfig;
pub use keys::{redis_key, validate_logical_key};
pub use marshaller::{BytesMarshaller, CacheMarshaller};
