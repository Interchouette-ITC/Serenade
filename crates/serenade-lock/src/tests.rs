//! Unit tests for `serenade-lock`.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::{
    DEFAULT_LOCK_TTL, InMemoryLockStore, LockError, LockFactory, LockKey, LockStore,
    generate_lock_token, validate_resource, version,
};

#[test]
fn version_is_nonempty() {
    assert_ne!(version(), "");
}

#[test]
fn validate_resource_rejects_empty() {
    assert!(matches!(
        validate_resource(""),
        Err(LockError::InvalidKey { .. })
    ));
    validate_resource("cron:mail").expect("ok");
}

#[test]
fn lock_key_accessors() {
    let key = LockKey::new("res", "tok").expect("key");
    assert_eq!(key.resource(), "res");
    assert_eq!(key.token(), "tok");
    assert!(LockKey::new("", "tok").is_err());
}

#[test]
fn generate_lock_token_is_hex() {
    let token = generate_lock_token().expect("rng");
    assert_eq!(token.len(), 32);
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn acquire_release_and_contention() {
    let store = Arc::new(InMemoryLockStore::new());
    let factory = LockFactory::new(Arc::clone(&store) as Arc<dyn LockStore>);
    let first = factory
        .create_lock("job:a", Some(Duration::from_secs(30)), false)
        .expect("first");
    let second = factory
        .create_lock("job:a", Some(Duration::from_secs(30)), false)
        .expect("second");

    assert!(first.acquire().expect("acquire1"));
    assert!(first.is_acquired().expect("held"));
    assert!(!second.acquire().expect("conflict"));
    first.release().expect("release");
    assert!(!first.is_acquired().expect("gone"));
    assert!(second.acquire().expect("acquire2"));
    second.release().expect("release2");
}

#[test]
fn reacquire_same_token_succeeds() {
    let factory = LockFactory::new(Arc::new(InMemoryLockStore::new()) as Arc<dyn LockStore>);
    let lock = factory.create_lock_forever("job:b", false).expect("lock");
    assert!(lock.acquire().expect("1"));
    assert!(lock.acquire().expect("2"));
    assert_eq!(lock.ttl(), None);
    assert_eq!(lock.resource(), "job:b");
    assert_ne!(lock.token(), "");
}

#[test]
fn ttl_expiry_frees_resource() {
    let factory = LockFactory::new(Arc::new(InMemoryLockStore::new()) as Arc<dyn LockStore>);
    let first = factory
        .create_lock("job:ttl", Some(Duration::from_millis(40)), false)
        .expect("first");
    let second = factory
        .create_lock("job:ttl", Some(Duration::from_secs(5)), false)
        .expect("second");
    assert!(first.acquire().expect("acquire"));
    thread::sleep(Duration::from_millis(60));
    assert!(!first.is_acquired().expect("expired"));
    assert!(second.acquire().expect("after expiry"));
    second.release().expect("release");
}

#[test]
fn refresh_extends_ttl() {
    let factory = LockFactory::new(Arc::new(InMemoryLockStore::new()) as Arc<dyn LockStore>);
    let lock = factory
        .create_lock("job:refresh", Some(Duration::from_millis(40)), false)
        .expect("lock");
    assert!(lock.acquire().expect("acquire"));
    thread::sleep(Duration::from_millis(25));
    lock.refresh(Some(Duration::from_millis(80)))
        .expect("refresh");
    thread::sleep(Duration::from_millis(40));
    assert!(lock.is_acquired().expect("still held"));
    lock.release().expect("release");
}

#[test]
fn refresh_without_hold_fails() {
    let factory = LockFactory::new(Arc::new(InMemoryLockStore::new()) as Arc<dyn LockStore>);
    let lock = factory
        .create_lock_forever("job:none", false)
        .expect("lock");
    assert!(matches!(lock.refresh(None), Err(LockError::NotHeld { .. })));
}

#[test]
fn create_lock_defaults_ttl() {
    let factory = LockFactory::new(Arc::new(InMemoryLockStore::new()) as Arc<dyn LockStore>);
    let lock = factory
        .create_lock("job:default", None, false)
        .expect("lock");
    assert_eq!(lock.ttl(), Some(DEFAULT_LOCK_TTL));
}

#[test]
fn empty_resource_rejected() {
    let factory = LockFactory::new(Arc::new(InMemoryLockStore::new()) as Arc<dyn LockStore>);
    assert!(matches!(
        factory.create_lock("", None, false),
        Err(LockError::InvalidKey { .. })
    ));
}

#[test]
fn auto_release_on_drop() {
    let store = Arc::new(InMemoryLockStore::new());
    let factory = LockFactory::new(Arc::clone(&store) as Arc<dyn LockStore>);
    {
        let lock = factory.create_lock_forever("job:drop", true).expect("lock");
        assert!(lock.acquire().expect("acquire"));
    }
    let next = factory
        .create_lock_forever("job:drop", false)
        .expect("next");
    assert!(next.acquire().expect("after drop"));
    next.release().expect("release");
}

#[test]
fn store_delete_other_token_is_idempotent() {
    let store = InMemoryLockStore::new();
    store
        .save("r", "a", Some(Duration::from_secs(10)))
        .expect("save");
    store.delete("r", "b").expect("noop");
    assert!(store.exists("r", "a").expect("still a"));
}

#[test]
fn put_off_expiration_clears_ttl() {
    let store = InMemoryLockStore::new();
    store
        .save("r", "a", Some(Duration::from_millis(30)))
        .expect("save");
    store.put_off_expiration("r", "a", None).expect("forever");
    thread::sleep(Duration::from_millis(40));
    assert!(store.exists("r", "a").expect("still held"));
}
