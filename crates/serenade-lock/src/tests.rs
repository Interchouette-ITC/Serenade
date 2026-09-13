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

#[test]
fn delete_expired_entry_and_put_off_guards() {
    let store = InMemoryLockStore::new();
    store
        .save("exp", "a", Some(Duration::from_millis(25)))
        .expect("save");
    thread::sleep(Duration::from_millis(40));
    store.delete("exp", "a").expect("delete expired");
    assert!(!store.exists("exp", "a").expect("gone"));

    store
        .save("exp2", "a", Some(Duration::from_millis(25)))
        .expect("save2");
    thread::sleep(Duration::from_millis(40));
    assert!(matches!(
        store.put_off_expiration("exp2", "a", Some(Duration::from_secs(1))),
        Err(LockError::NotHeld { .. })
    ));

    store
        .save("held", "owner", Some(Duration::from_secs(30)))
        .expect("save3");
    assert!(matches!(
        store.put_off_expiration("held", "other", None),
        Err(LockError::NotHeld { .. })
    ));
}

#[test]
fn auto_release_noop_when_never_acquired() {
    let factory = LockFactory::new(Arc::new(InMemoryLockStore::new()) as Arc<dyn LockStore>);
    let lock = factory.create_lock_forever("job:idle", true).expect("lock");
    drop(lock);
    let next = factory
        .create_lock_forever("job:idle", false)
        .expect("next");
    assert!(next.acquire().expect("free"));
    next.release().expect("release");
}

#[test]
fn acquire_propagates_non_conflict_store_errors() {
    struct BoomStore;

    impl LockStore for BoomStore {
        fn save(
            &self,
            _resource: &str,
            _token: &str,
            _ttl: Option<Duration>,
        ) -> Result<(), LockError> {
            Err(LockError::Store {
                message: "boom".to_owned(),
            })
        }

        fn delete(&self, _resource: &str, _token: &str) -> Result<(), LockError> {
            Ok(())
        }

        fn exists(&self, _resource: &str, _token: &str) -> Result<bool, LockError> {
            Ok(false)
        }

        fn put_off_expiration(
            &self,
            _resource: &str,
            _token: &str,
            _ttl: Option<Duration>,
        ) -> Result<(), LockError> {
            Ok(())
        }
    }

    let factory = LockFactory::new(Arc::new(BoomStore) as Arc<dyn LockStore>);
    let lock = factory
        .create_lock_forever("job:boom", false)
        .expect("lock");
    assert!(matches!(
        lock.acquire(),
        Err(LockError::Store { message }) if message == "boom"
    ));
}

#[test]
fn filesystem_lock_store_contention_and_ttl() {
    use crate::{FilesystemLockStore, FilesystemLockStoreConfig};

    let dir = tempfile::tempdir().expect("tempdir");
    let store =
        FilesystemLockStore::open(FilesystemLockStoreConfig::new(dir.path()).with_prefix("locks"))
            .expect("open");
    let factory = LockFactory::new(Arc::new(store) as Arc<dyn LockStore>);
    let first = factory
        .create_lock("job:fs", Some(Duration::from_secs(30)), false)
        .expect("first");
    let second = factory
        .create_lock("job:fs", Some(Duration::from_secs(30)), false)
        .expect("second");
    assert!(first.acquire().expect("acquire"));
    assert!(!second.acquire().expect("conflict"));
    first.release().expect("release");
    assert!(second.acquire().expect("after release"));
    second.release().expect("release2");
}

#[test]
fn filesystem_config_accessors_and_bad_prefix() {
    use crate::{FilesystemLockStore, FilesystemLockStoreConfig};

    let dir = tempfile::tempdir().expect("tempdir");
    let config = FilesystemLockStoreConfig::new(dir.path()).with_prefix("acc");
    assert_eq!(config.directory(), dir.path());
    assert_eq!(config.prefix(), "acc");
    assert!(FilesystemLockStore::open(config).is_ok());
    assert!(
        FilesystemLockStore::open(FilesystemLockStoreConfig::new(dir.path()).with_prefix("../x"))
            .is_err()
    );
}

#[test]
fn compile_pass_registers_in_memory_store() {
    use serenade_di::{CompilePass, ContainerBuilder};

    use crate::{DEFAULT_LOCK_STORE_SERVICE, LockStoreService, RegisterDefaultLockPass};

    let pass = RegisterDefaultLockPass;
    assert_eq!(pass.name(), "register_default_lock_store");
    let mut builder = ContainerBuilder::new();
    builder.add_compile_pass(RegisterDefaultLockPass);
    let container = builder.compile().expect("compile");
    let service = container
        .get_as::<LockStoreService>(DEFAULT_LOCK_STORE_SERVICE)
        .expect("lock.store");
    let lock = service
        .factory()
        .create_lock_forever("job:di", false)
        .expect("lock");
    assert!(lock.acquire().expect("acquire"));
    lock.release().expect("release");
}
