//! Redis list transport (feature `redis`).

mod adapter;
mod config;

pub use adapter::RedisTransport;
pub use config::RedisTransportConfig;
