//! Connection settings for [`super::RedisAdapter`].

use std::time::Duration;

/// Configuration for a Redis-backed cache pool.
#[derive(Debug, Clone)]
pub struct RedisAdapterConfig {
    /// Redis URL (`redis://127.0.0.1:6379/0`).
    pub url: String,
    /// Prefix prepended to every logical key (default `serenade:`).
    pub prefix: String,
    /// Maximum r2d2 pool size (default `16`).
    pub pool_max_size: u32,
    /// How long to wait for a pooled connection (default 5s).
    pub connection_timeout: Duration,
    /// When true, create one idle connection at build time (default false).
    pub warm_pool: bool,
}

impl Default for RedisAdapterConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379/0".to_owned(),
            prefix: "serenade:".to_owned(),
            pool_max_size: 16,
            connection_timeout: Duration::from_secs(5),
            warm_pool: false,
        }
    }
}

impl RedisAdapterConfig {
    /// Config for `url` with default prefix and pool size.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Self::default()
        }
    }

    /// Sets the key prefix.
    #[must_use]
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
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

#[cfg(test)]
mod tests {
    use super::RedisAdapterConfig;
    use std::time::Duration;

    #[test]
    fn builders_override_defaults() {
        let config = RedisAdapterConfig::new("redis://example:6379/1")
            .with_prefix("app:")
            .with_pool_max_size(0)
            .with_connection_timeout(Duration::from_millis(100));
        assert_eq!(config.url, "redis://example:6379/1");
        assert_eq!(config.prefix, "app:");
        assert_eq!(config.pool_max_size, 1);
        assert_eq!(config.connection_timeout, Duration::from_millis(100));
    }
}
