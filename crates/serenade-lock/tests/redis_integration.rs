//! Redis lock store smoke tests (requires a live Redis).

#![cfg(feature = "redis")]

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serenade_lock::{LockError, LockFactory, LockStore, RedisLockStore, RedisLockStoreConfig};

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_owned())
}

fn connect_with(prefix: &str) -> RedisLockStore {
    RedisLockStore::connect(RedisLockStoreConfig::new(redis_url()).with_prefix(prefix))
        .expect("redis")
}

#[test]
fn invalid_url_fails_connect() {
    let result = RedisLockStore::connect(RedisLockStoreConfig::new("not-a-redis-url"));
    assert!(matches!(result, Err(LockError::Store { .. })));
}

#[test]
fn warm_pool_against_down_host_fails_connect() {
    let config = RedisLockStoreConfig::new("redis://127.0.0.1:1/0")
        .with_pool_max_size(1)
        .with_connection_timeout(Duration::from_millis(200))
        .with_warm_pool(true);
    assert!(RedisLockStore::connect(config).is_err());
}

#[test]
fn warm_pool_connects_against_live_redis() {
    let store = RedisLockStore::connect(
        RedisLockStoreConfig::new(redis_url())
            .with_prefix("serenade:test-lock-warm:")
            .with_warm_pool(true)
            .with_pool_max_size(2),
    )
    .expect("warm redis");
    store
        .save("warm", "tok", Some(Duration::from_secs(5)))
        .expect("save");
    assert!(store.exists("warm", "tok").expect("exists"));
    store.delete("warm", "tok").expect("delete");
}

#[test]
fn redis_lock_acquire_release_and_conflict() {
    let store = Arc::new(connect_with("serenade:test-lock:"));
    let factory = LockFactory::new(Arc::clone(&store) as Arc<dyn LockStore>);
    let resource = format!("job:redis:{}", std::process::id());
    let first = factory
        .create_lock(&resource, Some(Duration::from_secs(30)), false)
        .expect("first");
    let second = factory
        .create_lock(&resource, Some(Duration::from_secs(30)), false)
        .expect("second");
    assert!(first.acquire().expect("acquire"));
    assert!(first.is_acquired().expect("held"));
    assert!(!second.acquire().expect("conflict"));
    first.release().expect("release");
    assert!(second.acquire().expect("after"));
    second.release().expect("release2");
}

#[test]
fn redis_reacquire_exists_refresh_and_delete_guards() {
    let store = Arc::new(connect_with("serenade:test-lock-refresh:"));
    let factory = LockFactory::new(Arc::clone(&store) as Arc<dyn LockStore>);
    let resource = format!("job:refresh:{}", std::process::id());
    let lock = factory
        .create_lock(&resource, Some(Duration::from_secs(30)), false)
        .expect("lock");
    assert!(lock.acquire().expect("1"));
    assert!(lock.acquire().expect("reacquire"));
    assert!(lock.is_acquired().expect("exists"));
    lock.refresh(Some(Duration::from_secs(60)))
        .expect("refresh ttl");
    lock.refresh(None).expect("refresh default ttl");

    let forever = factory
        .create_lock_forever(format!("job:forever:{}", std::process::id()), false)
        .expect("forever");
    assert!(forever.acquire().expect("forever acquire"));
    forever.refresh(None).expect("persist");
    forever.release().expect("forever release");

    store
        .delete(&resource, "other-token")
        .expect("wrong token noop");
    assert!(matches!(
        store.put_off_expiration("missing-resource", "tok", None),
        Err(LockError::NotHeld { .. })
    ));
    lock.release().expect("release");
}

#[test]
fn redis_contention_race_covers_nx_miss() {
    let store = Arc::new(connect_with("serenade:test-lock-race:"));
    let factory = LockFactory::new(Arc::clone(&store) as Arc<dyn LockStore>);
    let resource = format!("job:race:{}", std::process::id());
    let locks: Vec<_> = (0..16)
        .map(|_| {
            factory
                .create_lock(&resource, Some(Duration::from_secs(10)), false)
                .expect("lock")
        })
        .collect();

    let acquired = thread::scope(|scope| {
        let handles: Vec<_> = locks
            .iter()
            .map(|lock| scope.spawn(|| lock.acquire().expect("acquire")))
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("join"))
            .filter(|won| *won)
            .count()
    });
    assert_eq!(acquired, 1);
    for lock in &locks {
        let _ = lock.release();
    }
}
