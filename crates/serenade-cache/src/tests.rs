use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serenade_di::{CompilePass, ContainerBuilder, ServiceDefinition};

use super::{
    ArrayAdapter, ArrayCacheItem, CACHE_POOL_TAG, CacheError, CacheItem, CacheItemPool,
    CachePoolService, DEFAULT_CACHE_POOL_SERVICE, RegisterDefaultCachePoolPass, version,
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

#[test]
fn tag_invalidation_removes_matching_keys() {
    let pool = ArrayAdapter::new();

    let mut a = ArrayCacheItem::miss("product:1");
    a.set(Arc::new(String::from("one")));
    a.tag(&["product", "catalog"]);
    pool.save(a).expect("save a");

    let mut b = ArrayCacheItem::miss("product:2");
    b.set(Arc::new(String::from("two")));
    b.tag(&["product"]);
    pool.save(b).expect("save b");

    let mut c = ArrayCacheItem::miss("user:1");
    c.set(Arc::new(String::from("user")));
    c.tag(&["user"]);
    pool.save(c).expect("save c");

    let hit = pool.get_item("product:1").expect("get");
    assert_eq!(hit.tags(), &["product".to_owned(), "catalog".to_owned()]);

    assert_eq!(pool.invalidate_tags(&["product"]).expect("invalidate"), 2);
    assert!(!pool.get_item("product:1").expect("get").is_hit());
    assert!(!pool.get_item("product:2").expect("get").is_hit());
    assert!(pool.get_item("user:1").expect("get").is_hit());

    assert_eq!(pool.invalidate_tags(&["missing"]).expect("noop"), 0);
    assert_eq!(pool.invalidate_tags(&[""]).expect("empty tag"), 0);
}

#[test]
fn expiry_unlinks_last_tag_and_rejects_stale_save() {
    let pool = ArrayAdapter::new();
    let mut item = ArrayCacheItem::miss("tmp");
    item.set(Arc::new(1_u8));
    item.tag(&["ephemeral"]);
    item.expires_after(Some(Duration::from_millis(20)));
    pool.save(item).expect("save");
    thread::sleep(Duration::from_millis(40));
    assert!(!pool.get_item("tmp").expect("get").is_hit());
    assert_eq!(pool.invalidate_tags(&["ephemeral"]).expect("gone"), 0);

    let mut stale = ArrayCacheItem::miss("stale");
    stale.set(Arc::new(2_u8));
    stale.expires_after(Some(Duration::ZERO));
    pool.save(stale).expect("expired save");
    assert!(!pool.get_item("stale").expect("get").is_hit());
}

#[test]
fn tag_dedupes_and_survives_overwrite_without_tags() {
    let pool = ArrayAdapter::new();
    let mut item = ArrayCacheItem::miss("k");
    item.set(Arc::new(1_u8));
    item.tag(&["a", "a", ""]);
    item.tag(&["b"]);
    assert_eq!(item.tags(), &["a".to_owned(), "b".to_owned()]);
    pool.save(item).expect("save");

    let mut overwrite = ArrayCacheItem::miss("k");
    overwrite.set(Arc::new(2_u8));
    pool.save(overwrite).expect("overwrite clears tags");
    assert_eq!(pool.get_item("k").expect("get").tags(), &[] as &[String]);
    assert_eq!(pool.invalidate_tags(&["a"]).expect("gone"), 0);
}

#[test]
fn filesystem_and_default_reject_tag_invalidation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let pool = super::FilesystemAdapter::open(
        super::FilesystemAdapterConfig::new(dir.path()),
        Arc::new(super::BytesMarshaller),
    )
    .expect("open");
    assert!(matches!(
        pool.invalidate_tags(&["x"]),
        Err(CacheError::Pool { .. })
    ));
}
