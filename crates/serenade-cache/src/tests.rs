use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serenade_di::{ContainerBuilder, ServiceDefinition};

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
fn empty_key_is_rejected() {
    let pool = ArrayAdapter::new();
    let Err(err) = pool.get_item("") else {
        panic!("empty key must fail");
    };
    assert!(matches!(err, CacheError::InvalidKey { .. }));
}

#[test]
fn compile_pass_registers_default_pool() {
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
fn compile_pass_aliases_tagged_pool() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(
            ServiceDefinition::new("cache.custom").with_tag(CACHE_POOL_TAG),
            |_c| Ok(Box::new(CachePoolService(Arc::new(ArrayAdapter::new())))),
        )
        .expect("register");
    builder.add_compile_pass(RegisterDefaultCachePoolPass);
    let container = builder.compile().expect("compile");
    let _ = container
        .get_as::<CachePoolService>(DEFAULT_CACHE_POOL_SERVICE)
        .expect("aliased");
}
