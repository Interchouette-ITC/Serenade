use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serenade_di::{CompilePass, ContainerBuilder, ServiceDefinition};

use super::{
    version, ArrayAdapter, ArrayCacheItem, CacheError, CacheItem, CacheItemPool, CachePoolService,
    RegisterDefaultCachePoolPass, CACHE_POOL_TAG, DEFAULT_CACHE_POOL_SERVICE,
};

#[test]
fn version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn miss_hit_overwrite_delete_clear() {
    let pool = ArrayAdapter::new();
    let miss = pool.get_item("sku").expect("get");
    assert!(!miss.is_hit());
    assert!(miss.get().is_none());
    assert_eq!(miss.key(), "sku");

    let mut item = ArrayCacheItem::miss("sku");
    item.set(Arc::new(String::from("A1")));
    pool.save(item).expect("save");

    let hit = pool.get_item("sku").expect("get");
    assert!(hit.is_hit());
    assert_eq!(hit.key(), "sku");
    assert_eq!(
        hit.get()
            .and_then(|value| value.downcast_ref::<String>())
            .map(String::as_str),
        Some("A1")
    );

    let mut item = ArrayCacheItem::miss("sku");
    item.set(Arc::new(String::from("B2")));
    pool.save(item).expect("overwrite");
    let hit = pool.get_item("sku").expect("get");
    assert_eq!(
        hit.get()
            .and_then(|value| value.downcast_ref::<String>())
            .map(String::as_str),
        Some("B2")
    );

    assert!(pool.delete_item("sku").expect("delete"));
    assert!(!pool.delete_item("sku").expect("missing"));
    assert!(!pool.get_item("sku").expect("get").is_hit());

    let mut item = ArrayCacheItem::miss("x");
    item.set(Arc::new(1_u32));
    pool.save(item).expect("save");
    pool.clear().expect("clear");
    assert!(!pool.get_item("x").expect("get").is_hit());
}

#[test]
fn expiry_turns_hit_into_miss() {
    let pool = ArrayAdapter::new();
    let mut item = ArrayCacheItem::miss("tmp");
    item.set(Arc::new(42_u64));
    item.expires_after(Some(Duration::from_millis(20)));
    pool.save(item).expect("save");
    assert!(pool.get_item("tmp").expect("get").is_hit());
    thread::sleep(Duration::from_millis(40));
    assert!(!pool.get_item("tmp").expect("get").is_hit());
}

#[test]
fn item_get_returns_none_when_expired() {
    let mut item = ArrayCacheItem::hit("k", Arc::new(1_u8));
    item.expires_after(Some(Duration::from_millis(10)));
    thread::sleep(Duration::from_millis(25));
    assert!(item.is_expired());
    assert!(!item.is_hit());
    assert!(item.get().is_none());
}

#[test]
fn save_without_value_is_miss_on_get() {
    let pool = ArrayAdapter::new();
    pool.save(ArrayCacheItem::miss("empty")).expect("save");
    let item = pool.get_item("empty").expect("get");
    assert!(!item.is_hit());
    assert!(item.get().is_none());
}

#[test]
fn empty_key_is_rejected() {
    let pool = ArrayAdapter::new();
    let Err(err) = pool.get_item("") else {
        panic!("empty key must fail");
    };
    assert!(matches!(err, CacheError::InvalidKey { .. }));
    let Err(err) = pool.save(ArrayCacheItem::miss("")) else {
        panic!("empty key save must fail");
    };
    assert!(matches!(err, CacheError::InvalidKey { .. }));
    let Err(err) = pool.delete_item("") else {
        panic!("empty key delete must fail");
    };
    assert!(matches!(err, CacheError::InvalidKey { .. }));
}

#[test]
fn compile_pass_name_and_default_pool() {
    let pass = RegisterDefaultCachePoolPass;
    assert_eq!(pass.name(), "register_default_cache_pool");
    let mut builder = ContainerBuilder::new();
    builder.add_compile_pass(RegisterDefaultCachePoolPass);
    let container = builder.compile().expect("compile");
    let pool = container
        .get_as::<CachePoolService>(DEFAULT_CACHE_POOL_SERVICE)
        .expect("pool");
    let mut item = ArrayCacheItem::miss("k");
    item.set(Arc::new(7_i32));
    pool.0.save(item).expect("save");
    assert!(pool.0.get_item("k").expect("get").is_hit());
}

#[test]
fn compile_pass_skips_when_default_already_registered() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(
            ServiceDefinition::new(DEFAULT_CACHE_POOL_SERVICE).with_tag(CACHE_POOL_TAG),
            |_c| Ok(Box::new(CachePoolService(Arc::new(ArrayAdapter::new())))),
        )
        .expect("register");
    builder.add_compile_pass(RegisterDefaultCachePoolPass);
    let container = builder.compile().expect("compile");
    let _ = container
        .get_as::<CachePoolService>(DEFAULT_CACHE_POOL_SERVICE)
        .expect("existing default");
}

#[test]
fn expires_after_none_clears_ttl() {
    let mut item = ArrayCacheItem::hit("k", Arc::new(9_u8));
    item.expires_after(Some(Duration::from_secs(60)));
    item.expires_after(None);
    assert!(!item.is_expired());
    assert!(item.is_hit());
}

#[test]
fn has_item_and_batch_defaults() {
    let pool = ArrayAdapter::new();
    assert!(!pool.has_item("a").expect("has"));
    let mut item = ArrayCacheItem::miss("a");
    item.set(Arc::new(1_u8));
    pool.save(item).expect("save");
    assert!(pool.has_item("a").expect("has"));
    let items = pool.get_items(&["a", "missing"]).expect("batch get");
    assert_eq!(items.len(), 2);
    assert!(items[0].is_hit());
    assert!(!items[1].is_hit());
    assert_eq!(pool.delete_items(&["a", "missing"]).expect("batch del"), 1);
}
