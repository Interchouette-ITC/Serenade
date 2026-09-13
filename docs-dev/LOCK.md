# Lock

Named locks live in **`serenade-lock`** ([#185](https://github.com/Interchouette-ITC/Serenade/issues/185), [#190](https://github.com/Interchouette-ITC/Serenade/issues/190), [#191](https://github.com/Interchouette-ITC/Serenade/issues/191)).

Symfony **Lock** analogue for critical sections (cron, inventory, mail blast). Apps own lock key naming.

## Types

| Type | Role |
| --- | --- |
| `LockStore` | Persist ownership (`save` / `delete` / `exists` / `put_off_expiration`) |
| `LockFactory` / `Lock` | Create handles; `acquire` / `release` / `refresh` / `is_acquired` |
| `LockKey` | Resource name + opaque owner token |
| `LockError` | Invalid key, conflict, not held, generation, store failures |

## Stores

| Store | Role |
| --- | --- |
| `InMemoryLockStore` | Process-local (default DI) |
| `FilesystemLockStore` | Disk files under `{directory}/{prefix}/{hex}.lock` |
| `RedisLockStore` | Redis via redis-rs + r2d2 (Cargo feature `redis`) |

## DI

`FrameworkExtension` adds [`RegisterDefaultLockPass`], which registers service id `lock.store` (`DEFAULT_LOCK_STORE_SERVICE`) tagged `lock.store` with an `InMemoryLockStore` when missing. Resolve `LockStoreService` and call `factory()`.

Apps replace the default by registering filesystem or Redis stores under the same id before compile.

## Example

```rust
use std::sync::Arc;
use std::time::Duration;
use serenade_lock::{InMemoryLockStore, LockFactory, LockStore};

let store = Arc::new(InMemoryLockStore::new());
let factory = LockFactory::new(store);
let lock = factory.create_lock("cron:mail-blast", Some(Duration::from_secs(30)), true)?;
if lock.acquire()? {
    // critical section
    lock.release()?;
}
```

Filesystem:

```rust
use serenade_lock::{FilesystemLockStore, FilesystemLockStoreConfig};

let store = FilesystemLockStore::open(
    FilesystemLockStoreConfig::new("var/lock").with_prefix("serenade"),
)?;
```

Redis (feature `redis`):

```rust
use serenade_lock::{RedisLockStore, RedisLockStoreConfig};

let store = RedisLockStore::connect(
    RedisLockStoreConfig::new("redis://127.0.0.1:6379/0").with_prefix("serenade:lock:"),
)?;
```

## Related

- Parent epic: [#185](https://github.com/Interchouette-ITC/Serenade/issues/185)
- Cache Redis patterns: [CACHE.md](CACHE.md)
