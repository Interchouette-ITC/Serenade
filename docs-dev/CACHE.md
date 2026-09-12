# Cache component

PSR-like cache pools live in **`serenade-cache`**.

| Piece | Role |
| --- | --- |
| `CacheItem` / `ArrayCacheItem` | Key, hit flag, `Arc` value, optional TTL, optional tags |
| `CacheItemPool` / `ArrayAdapter` | In-memory get/save/delete/clear; **tag invalidation** |
| `FilesystemAdapter` | Disk-backed pool (directory + prefix); TTL in file header |
| `CacheMarshaller` / `BytesMarshaller` | Wire encoding (`Vec<u8>`, `String`) for durable adapters |
| `RedisAdapter` (feature `redis`) | Redis via [redis-rs](https://github.com/redis-rs/redis-rs) + r2d2 |
| `cache.pool` tag | DI tag; `RegisterDefaultCachePoolPass` seeds `cache.app` with `ArrayAdapter` |

## Tag invalidation

Symfony-shaped tags on items, invalidation on the pool:

```rust
use std::sync::Arc;
use serenade_cache::{ArrayAdapter, ArrayCacheItem, CacheItem, CacheItemPool};

let pool = ArrayAdapter::new();
let mut item = ArrayCacheItem::miss("product:42");
item.set(Arc::new(String::from("payload")));
item.tag(&["product", "catalog"]);
pool.save(item)?;

pool.invalidate_tags(&["product"])?; // removes every item tagged `product`
```

- Empty tag names are ignored when tagging or invalidating
- Re-saving an item **without** tags clears its previous tag links on `ArrayAdapter`
- `FilesystemAdapter` and `RedisAdapter` do not persist tags yet; `invalidate_tags` returns a pool error on those adapters

## Filesystem adapter

No feature flag. Open with a directory and optional prefix subdirectory (default `serenade`):

```rust
use std::sync::Arc;
use serenade_cache::{
    BytesMarshaller, CacheItemPool, FilesystemAdapter, FilesystemAdapterConfig,
};

let pool = FilesystemAdapter::open(
    FilesystemAdapterConfig::new("/var/cache/myapp").with_prefix("serenade"),
    Arc::new(BytesMarshaller),
)?;
```

- One file per key under `{directory}/{prefix}/{hex(key)}.cache`
- File header stores version + absolute expiry (Unix millis); `0` means no TTL
- Expired items are **deleted on read** (`get_item` / miss path)
- `clear` removes `*.cache` files in the prefix directory (not the whole disk)
- Values must be marshallable (`BytesMarshaller`: `String` / `Vec<u8>`); other types: serialize in the app first
- Atomic write via temp file + rename

Default DI pass still registers **`ArrayAdapter`**. Apps register `FilesystemAdapter` (or Redis) themselves under `cache.pool` / `cache.app`.

## Redis adapter

Enable with `--features redis` on `serenade-cache`.

- URL: `RedisAdapterConfig::new("redis://127.0.0.1:6379/0")`
- Logical key `cart:1` becomes Redis key `{prefix}cart:1` (default prefix `serenade:`)
- TTL uses `SET … PX`
- `clear` runs `SCAN MATCH {prefix}*` then `DEL` (never `FLUSHDB`)
- Uses the same `BytesMarshaller` as the filesystem adapter

`CacheItemPool` is **sync**. Async HTTP callers should use `spawn_blocking` (or call from sync code).

### Local Redis

```text
make redis-up
make redis-test
make redis-down
```

Compose file: `docker/compose.yml` (`redis:7-alpine` on port `6379`). Override with `REDIS_URL`.

### Values

`BytesMarshaller` stores tagged `Vec<u8>` and `String` payloads. Other types: serialize in the app, then `CacheItem::set(Arc::new(bytes))`.
