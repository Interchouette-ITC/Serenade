# serenade-cache

PSR-like cache contracts, in-memory `ArrayAdapter`, disk `FilesystemAdapter`, and
optional `RedisAdapter` (Cargo feature `redis`: redis-rs + r2d2).

Default DI pool is in-memory unless the app registers Filesystem or Redis.
