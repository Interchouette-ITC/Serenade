//! Redis lock store smoke tests (requires a live Redis).

#![cfg(feature = "redis")]

use std::sync::Arc;
use std::time::Duration;

use serenade_lock::{LockFactory, LockStore, RedisLockStore, RedisLockStoreConfig};

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_owned())
}

fn connect() -> RedisLockStore {
    RedisLockStore::connect(
        RedisLockStoreConfig::new(redis_url()).with_prefix("serenade:test-lock:"),
    )
    .expect("redis")
}

#[test]
fn redis_lock_acquire_release_and_conflict() {
    let store = Arc::new(connect());
    let factory = LockFactory::new(Arc::clone(&store) as Arc<dyn LockStore>);
    let resource = format!("job:redis:{}", std::process::id());
    let first = factory
        .create_lock(&resource, Some(Duration::from_secs(30)), false)
        .expect("first");
    let second = factory
        .create_lock(&resource, Some(Duration::from_secs(30)), false)
        .expect("second");
    assert!(first.acquire().expect("acquire"));
    assert!(!second.acquire().expect("conflict"));
    first.release().expect("release");
    assert!(second.acquire().expect("after"));
    second.release().expect("release2");
}
