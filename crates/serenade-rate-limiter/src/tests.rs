//! Unit tests for `serenade-rate-limiter`.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::{
    InMemoryRateLimiterStorage, Policy, RateLimiterError, RateLimiterFactory, RateLimiterStorage,
    validate_key, version,
};

#[test]
fn version_is_nonempty() {
    assert_ne!(version(), "");
}

#[test]
fn validate_key_rejects_empty() {
    assert!(matches!(
        validate_key(""),
        Err(RateLimiterError::InvalidKey { .. })
    ));
    validate_key("login").expect("ok");
}

#[test]
fn policy_builders_reject_zero() {
    assert!(Policy::token_bucket(0, Duration::from_secs(1)).is_err());
    assert!(Policy::token_bucket(5, Duration::ZERO).is_err());
    assert!(Policy::fixed_window(0, Duration::from_secs(1)).is_err());
    assert!(Policy::fixed_window(5, Duration::ZERO).is_err());
}

#[test]
fn token_bucket_accepts_then_rejects_until_refill() {
    let storage = Arc::new(InMemoryRateLimiterStorage::new());
    let factory = RateLimiterFactory::new(
        "api",
        Policy::token_bucket(2, Duration::from_millis(80)).expect("policy"),
        storage as Arc<dyn RateLimiterStorage>,
    )
    .expect("factory");
    let limiter = factory.create("user-1").expect("limiter");
    assert_eq!(limiter.id(), "api-user-1");
    assert_eq!(limiter.policy().limit(), 2);

    let first = limiter.consume(1).expect("1");
    assert!(first.is_accepted());
    assert_eq!(first.remaining_tokens(), 1);
    assert_eq!(first.limit(), 2);

    let second = limiter.consume(1).expect("2");
    assert!(second.is_accepted());
    assert_eq!(second.remaining_tokens(), 0);

    let blocked = limiter.consume(1).expect("3");
    assert!(!blocked.is_accepted());
    assert!(blocked.retry_after().is_some());

    thread::sleep(Duration::from_millis(90));
    let after = limiter.consume(1).expect("4");
    assert!(after.is_accepted());
}

#[test]
fn token_bucket_burst_consume() {
    let storage = Arc::new(InMemoryRateLimiterStorage::new());
    let factory = RateLimiterFactory::new(
        "burst",
        Policy::token_bucket(5, Duration::from_secs(1)).expect("policy"),
        storage as Arc<dyn RateLimiterStorage>,
    )
    .expect("factory");
    let limiter = factory.create("ip").expect("limiter");
    let hit = limiter.consume(5).expect("burst");
    assert!(hit.is_accepted());
    assert_eq!(hit.remaining_tokens(), 0);
    assert!(!limiter.consume(1).expect("over").is_accepted());
}

#[test]
fn fixed_window_resets_after_interval() {
    let storage = Arc::new(InMemoryRateLimiterStorage::new());
    let factory = RateLimiterFactory::new(
        "login",
        Policy::fixed_window(3, Duration::from_millis(60)).expect("policy"),
        storage as Arc<dyn RateLimiterStorage>,
    )
    .expect("factory");
    let limiter = factory.create("alice").expect("limiter");

    assert!(limiter.consume(2).expect("a").is_accepted());
    assert!(limiter.consume(1).expect("b").is_accepted());
    let blocked = limiter.consume(1).expect("c");
    assert!(!blocked.is_accepted());
    assert_eq!(blocked.remaining_tokens(), 0);

    thread::sleep(Duration::from_millis(70));
    let next = limiter.consume(3).expect("d");
    assert!(next.is_accepted());
    assert_eq!(next.remaining_tokens(), 0);
}

#[test]
fn reset_clears_budget() {
    let storage = Arc::new(InMemoryRateLimiterStorage::new());
    let factory = RateLimiterFactory::new(
        "form",
        Policy::fixed_window(1, Duration::from_secs(30)).expect("policy"),
        storage as Arc<dyn RateLimiterStorage>,
    )
    .expect("factory");
    let limiter = factory.create("sess").expect("limiter");
    assert!(limiter.consume(1).expect("use").is_accepted());
    assert!(!limiter.consume(1).expect("block").is_accepted());
    limiter.reset().expect("reset");
    assert!(limiter.consume(1).expect("again").is_accepted());
}

#[test]
fn factory_rejects_empty_name_or_key() {
    let storage = Arc::new(InMemoryRateLimiterStorage::new()) as Arc<dyn RateLimiterStorage>;
    assert!(matches!(
        RateLimiterFactory::new(
            "",
            Policy::fixed_window(1, Duration::from_secs(1)).expect("policy"),
            Arc::clone(&storage),
        ),
        Err(RateLimiterError::InvalidKey { .. })
    ));
    let factory = RateLimiterFactory::new(
        "ok",
        Policy::fixed_window(1, Duration::from_secs(1)).expect("policy"),
        storage,
    )
    .expect("factory");
    assert_eq!(factory.name(), "ok");
    assert_eq!(factory.policy().limit(), 1);
    assert!(matches!(
        factory.create(""),
        Err(RateLimiterError::InvalidKey { .. })
    ));
}

#[test]
fn consume_zero_tokens_is_error() {
    let storage = Arc::new(InMemoryRateLimiterStorage::new());
    let factory = RateLimiterFactory::new(
        "z",
        Policy::token_bucket(3, Duration::from_secs(1)).expect("policy"),
        storage as Arc<dyn RateLimiterStorage>,
    )
    .expect("factory");
    let limiter = factory.create("k").expect("limiter");
    assert!(matches!(
        limiter.consume(0),
        Err(RateLimiterError::InvalidTokens { tokens: 0, .. })
    ));
}

#[test]
fn subjects_are_isolated() {
    let storage = Arc::new(InMemoryRateLimiterStorage::new());
    let factory = RateLimiterFactory::new(
        "iso",
        Policy::fixed_window(1, Duration::from_secs(30)).expect("policy"),
        storage as Arc<dyn RateLimiterStorage>,
    )
    .expect("factory");
    let a = factory.create("a").expect("a");
    let b = factory.create("b").expect("b");
    assert!(a.consume(1).expect("a1").is_accepted());
    assert!(b.consume(1).expect("b1").is_accepted());
    assert!(!a.consume(1).expect("a2").is_accepted());
}

#[test]
fn storage_ttl_expiry_clears_state() {
    use std::time::Instant;

    let storage = InMemoryRateLimiterStorage::new();
    let policy = Policy::fixed_window(1, Duration::from_secs(30)).expect("policy");
    let now = Instant::now();
    assert!(
        storage
            .consume("ttl-key", &policy, 1, now, Some(Duration::from_millis(20)))
            .expect("save")
            .is_accepted()
    );
    thread::sleep(Duration::from_millis(40));
    let again = storage
        .consume(
            "ttl-key",
            &policy,
            1,
            Instant::now(),
            Some(Duration::from_secs(30)),
        )
        .expect("after ttl");
    assert!(again.is_accepted());
}
