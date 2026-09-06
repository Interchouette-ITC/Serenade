# Cache component

PSR-like cache pools live in **`serenade-cache`**.

| Piece | Role |
| --- | --- |
| `CacheItem` / `ArrayCacheItem` | Key, hit flag, `Arc` value, optional TTL |
| `CacheItemPool` / `ArrayAdapter` | In-memory get/save/delete/clear |
| `RedisAdapter` (feature `redis`) | Redis via [redis-rs](https://github.com/redis-rs/redis-rs) + r2d2 |
| `CacheMarshaller` / `BytesMarshaller` | Wire encoding (`Vec<u8>`, `String`) |
| `cache.pool` tag | DI tag; `RegisterDefaultCachePoolPass` seeds `cache.app` with `ArrayAdapter` |

## Redis adapter

Enable with `--features redis` on `serenade-cache`.

- URL: `RedisAdapterConfig::new("redis://127.0.0.1:6379/0")`
- Logical key `cart:1` becomes Redis key `{prefix}cart:1` (default prefix `serenade:`)
- TTL uses `SET … PX`
- `clear` runs `SCAN MATCH {prefix}*` then `DEL` (never `FLUSHDB`)
- Default DI pass still registers **`ArrayAdapter`**; apps register `RedisAdapter` themselves under `cache.pool` / `cache.app`

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
