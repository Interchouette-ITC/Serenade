# serenade-lock

Symfony-shaped named locks: `LockStore`, `LockFactory` / `Lock`,
`InMemoryLockStore`, `FilesystemLockStore`, optional `RedisLockStore`
(Cargo feature `redis`), and DI via `RegisterDefaultLockPass`
(service id `lock.store`).

See `docs-dev/LOCK.md`.
