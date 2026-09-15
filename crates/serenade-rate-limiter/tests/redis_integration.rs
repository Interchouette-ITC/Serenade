//! Redis integration for `serenade-rate-limiter` (requires a live Redis).

#![cfg(feature = "redis")]

use std::sync::Arc;
use std::time::Duration;

use redis::Commands;
use serenade_rate_limiter::{
    Policy, RateLimiterError, RateLimiterFactory, RateLimiterStorage, RedisRateLimiterStorage,
    RedisRateLimiterStorageConfig,
};

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_owned())
}

fn unique_prefix(label: &str) -> String {
    format!(
        "serenade:rate-limiter:test:{label}:{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    )
}

fn connect(prefix: &str) -> RedisRateLimiterStorage {
    RedisRateLimiterStorage::connect(
        RedisRateLimiterStorageConfig::new(redis_url())
            .with_prefix(prefix)
            .with_pool_max_size(2)
            .with_connection_timeout(Duration::from_secs(2))
            .with_warm_pool(false),
    )
    .expect("connect")
}

#[test]
fn config_builders_and_prefix() {
    let config = RedisRateLimiterStorageConfig::new("redis://127.0.0.1:6379/0")
        .with_prefix("serenade:rate-limiter:cfg:")
        .with_pool_max_size(0)
        .with_connection_timeout(Duration::from_millis(500))
        .with_warm_pool(true);
    assert_eq!(config.pool_max_size, 1);
    assert!(config.warm_pool);
    assert_eq!(config.connection_timeout, Duration::from_millis(500));
    assert_eq!(config.prefix, "serenade:rate-limiter:cfg:");
}

#[test]
fn invalid_url_and_warm_pool_down_host_fail_connect() {
    assert!(matches!(
        RedisRateLimiterStorage::connect(RedisRateLimiterStorageConfig::new("not-a-redis-url")),
        Err(RateLimiterError::Storage { .. })
    ));
    let config = RedisRateLimiterStorageConfig::new("redis://127.0.0.1:1/0")
        .with_pool_max_size(1)
        .with_connection_timeout(Duration::from_millis(200))
        .with_warm_pool(true);
    assert!(RedisRateLimiterStorage::connect(config).is_err());
}

#[test]
fn redis_fixed_window_consume_reset_and_shared_prefix() {
    let prefix = unique_prefix("fw");
    let storage = Arc::new(connect(&prefix));
    assert_eq!(storage.prefix(), prefix);
    let factory = RateLimiterFactory::new(
        "login",
        Policy::fixed_window(2, Duration::from_secs(60)).expect("policy"),
        Arc::clone(&storage) as Arc<dyn RateLimiterStorage>,
    )
    .expect("factory");
    let limiter = factory.create("alice").expect("limiter");
    assert!(limiter.consume(1).expect("1").is_accepted());
    assert!(limiter.consume(1).expect("2").is_accepted());
    assert!(!limiter.consume(1).expect("3").is_accepted());

    let peer = Arc::new(connect(&prefix));
    let peer_factory = RateLimiterFactory::new(
        "login",
        Policy::fixed_window(2, Duration::from_secs(60)).expect("policy"),
        peer as Arc<dyn RateLimiterStorage>,
    )
    .expect("peer factory");
    let peer_limiter = peer_factory.create("alice").expect("peer");
    assert!(!peer_limiter.consume(1).expect("peer").is_accepted());

    limiter.reset().expect("reset");
    assert!(limiter.consume(1).expect("after reset").is_accepted());
}

#[test]
fn redis_token_bucket_and_corrupt_value() {
    let prefix = unique_prefix("tb");
    let storage = Arc::new(connect(&prefix));
    let factory = RateLimiterFactory::new(
        "api",
        Policy::token_bucket(2, Duration::from_millis(50)).expect("policy"),
        Arc::clone(&storage) as Arc<dyn RateLimiterStorage>,
    )
    .expect("factory");
    let limiter = factory.create("bob").expect("limiter");
    assert!(limiter.consume(2).expect("burst").is_accepted());
    assert!(!limiter.consume(1).expect("block").is_accepted());

    let client = redis::Client::open(redis_url()).expect("client");
    let mut conn = client.get_connection().expect("conn");
    let key = format!("{prefix}{}", limiter.id());
    let _: () = conn.set(&key, "not-a-window").expect("set");
    let err = limiter.consume(1).expect_err("corrupt");
    assert!(matches!(err, RateLimiterError::Storage { .. }));
}

#[test]
fn warm_pool_connects_and_zero_tokens_error() {
    let prefix = unique_prefix("warm");
    let storage = RedisRateLimiterStorage::connect(
        RedisRateLimiterStorageConfig::new(redis_url())
            .with_prefix(&prefix)
            .with_warm_pool(true)
            .with_pool_max_size(2),
    )
    .expect("warm");
    let policy = Policy::fixed_window(1, Duration::from_secs(30)).expect("policy");
    assert!(matches!(
        storage.consume("k", &policy, 0, std::time::Instant::now(), None),
        Err(RateLimiterError::InvalidTokens { .. })
    ));
    assert!(matches!(
        storage.reset(""),
        Err(RateLimiterError::InvalidKey { .. })
    ));
}
