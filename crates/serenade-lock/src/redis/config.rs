//! [`RedisLockStoreConfig`].

use std::time::Duration;

/// Connection and key-prefix settings for [`super::RedisLockStore`].
#[derive(Clone, Debug)]
pub struct RedisLockStoreConfig {
    /// Redis URL (`redis://…`).
    pub url: String,
    /// Key prefix (default `serenade:lock:`).
    pub prefix: String,
    /// r2d2 max pool size.
    pub pool_max_size: u32,
    /// Checkout timeout.
    pub connection_timeout: Duration,
    /// When true, keep at least one idle connection.
    pub warm_pool: bool,
}

impl RedisLockStoreConfig {
    /// Builds a config for `url` with defaults.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            prefix: "serenade:lock:".to_owned(),
            pool_max_size: 8,
            connection_timeout: Duration::from_secs(2),
            warm_pool: false,
        }
    }

    /// Sets the Redis key prefix.
    #[must_use]
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }
}
