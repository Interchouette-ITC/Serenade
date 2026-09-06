# serenade-cache

PSR-like cache contracts, in-memory `ArrayAdapter`, and optional `RedisAdapter`
(Cargo feature `redis`: redis-rs + r2d2).

Default DI pool is in-memory unless the app registers Redis.
