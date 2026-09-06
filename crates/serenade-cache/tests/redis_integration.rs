//! Redis integration tests (feature `redis`, needs a live Redis).

#![cfg(feature = "redis")]

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serenade_cache::{
    ArrayCacheItem, BytesMarshaller, CacheItem, CacheItemPool, RedisAdapter, RedisAdapterConfig,
};

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_owned())
}

fn adapter(prefix: &str) -> RedisAdapter {
    let config = RedisAdapterConfig::new(redis_url())
        .with_prefix(prefix)
        .with_pool_max_size(4)
        .with_connection_timeout(Duration::from_secs(3));
    RedisAdapter::connect(config, Arc::new(BytesMarshaller)).expect("connect Redis")
}

#[test]
fn invalid_url_fails_connect() {
    let result = RedisAdapter::connect(
        RedisAdapterConfig::new("not-a-redis-url"),
        Arc::new(BytesMarshaller),
    );
    assert!(result.is_err(), "bad url must fail");
}

#[test]
fn save_get_delete_and_ttl() {
    let pool = adapter("serenade-it:ttl:");
    pool.clear().expect("clear");

    let mut item = ArrayCacheItem::miss("sku");
    item.set(Arc::new(String::from("A1")));
    pool.save(item).expect("save");
    let hit = pool.get_item("sku").expect("get");
    assert!(hit.is_hit());
    assert_eq!(
        hit.get()
            .and_then(|value| value.downcast_ref::<String>())
            .map(String::as_str),
        Some("A1")
    );
    assert!(pool.has_item("sku").expect("has"));

    let mut item = ArrayCacheItem::miss("tmp");
    item.set(Arc::new(vec![9_u8, 8]));
    item.expires_after(Some(Duration::from_millis(80)));
    pool.save(item).expect("save ttl");
    assert!(pool.get_item("tmp").expect("get").is_hit());
    thread::sleep(Duration::from_millis(120));
    assert!(!pool.get_item("tmp").expect("get expired").is_hit());

    assert!(pool.delete_item("sku").expect("delete"));
    assert!(!pool.get_item("sku").expect("gone").is_hit());
}

#[test]
fn clear_is_prefix_scoped() {
    let a = adapter("serenade-it:a:");
    let b = adapter("serenade-it:b:");
    a.clear().expect("clear a");
    b.clear().expect("clear b");

    let mut item = ArrayCacheItem::miss("k");
    item.set(Arc::new(String::from("keep")));
    b.save(item).expect("save b");

    let mut item = ArrayCacheItem::miss("k");
    item.set(Arc::new(String::from("wipe")));
    a.save(item).expect("save a");
    a.clear().expect("clear a only");

    assert!(!a.get_item("k").expect("a").is_hit());
    assert!(b.get_item("k").expect("b").is_hit());
    b.clear().expect("cleanup");
}

#[test]
fn batch_get_and_delete() {
    let pool = adapter("serenade-it:batch:");
    pool.clear().expect("clear");
    let mut item = ArrayCacheItem::miss("one");
    item.set(Arc::new(String::from("1")));
    pool.save(item).expect("save");
    let mut item = ArrayCacheItem::miss("two");
    item.set(Arc::new(vec![2_u8]));
    pool.save(item).expect("save");

    let items = pool.get_items(&["one", "two", "missing"]).expect("mget");
    assert!(items[0].is_hit());
    assert!(items[1].is_hit());
    assert!(!items[2].is_hit());
    assert_eq!(pool.delete_items(&["one", "two"]).expect("mdel"), 2);
}

#[test]
fn save_without_value_deletes() {
    let pool = adapter("serenade-it:empty:");
    pool.clear().expect("clear");
    let mut item = ArrayCacheItem::miss("x");
    item.set(Arc::new(String::from("v")));
    pool.save(item).expect("save");
    pool.save(ArrayCacheItem::miss("x"))
        .expect("delete via miss");
    assert!(!pool.get_item("x").expect("get").is_hit());
}

#[test]
fn pool_reuses_connections() {
    let pool = adapter("serenade-it:pool:");
    pool.clear().expect("clear");
    for i in 0..8 {
        let mut item = ArrayCacheItem::miss(format!("k{i}"));
        item.set(Arc::new(format!("v{i}")));
        pool.save(item).expect("save");
    }
    assert!(pool.has_item("k0").expect("has"));
    pool.clear().expect("cleanup");
}
