//! [`RedisTransportConfig`].

use std::time::Duration;

/// Connection and list-key settings for [`super::RedisTransport`].
#[derive(Clone, Debug)]
pub struct RedisTransportConfig {
    /// Redis URL (`redis://…`).
    pub url: String,
    /// Redis list key holding wire frames (default `serenade:messenger:default`).
    pub list_key: String,
    /// r2d2 max pool size.
    pub pool_max_size: u32,
    /// Checkout timeout.
    pub connection_timeout: Duration,
    /// When true, keep at least one idle connection.
    pub warm_pool: bool,
}

impl RedisTransportConfig {
    /// Builds a config for `url` with defaults.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            list_key: "serenade:messenger:default".to_owned(),
            pool_max_size: 8,
            connection_timeout: Duration::from_secs(2),
            warm_pool: false,
        }
    }

    /// Sets the Redis list key.
    #[must_use]
    pub fn with_list_key(mut self, list_key: impl Into<String>) -> Self {
        self.list_key = list_key.into();
        self
    }

    /// Sets the r2d2 max pool size (minimum 1).
    #[must_use]
    pub fn with_pool_max_size(mut self, pool_max_size: u32) -> Self {
        self.pool_max_size = pool_max_size.max(1);
        self
    }

    /// Sets how long to wait when checking out a connection.
    #[must_use]
    pub const fn with_connection_timeout(mut self, connection_timeout: Duration) -> Self {
        self.connection_timeout = connection_timeout;
        self
    }

    /// Warms the pool with one idle connection at build time.
    #[must_use]
    pub const fn with_warm_pool(mut self, warm_pool: bool) -> Self {
        self.warm_pool = warm_pool;
        self
    }
}
