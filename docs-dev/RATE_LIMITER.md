# RateLimiter

Rate limiting lives in **`serenade-rate-limiter`** ([#186](https://github.com/Interchouette-ITC/Serenade/issues/186), [#192](https://github.com/Interchouette-ITC/Serenade/issues/192), [#193](https://github.com/Interchouette-ITC/Serenade/issues/193)).

Symfony **RateLimiter** analogue for login / API / form abuse. Serenade owns policies and storage; apps choose which keys and routes to limit.

## Types

| Type | Role |
| --- | --- |
| `Policy` | Token bucket or fixed window (`limit` + `interval`) |
| `RateLimiterFactory` / `RateLimiter` | Create per-subject handles; `consume` / `reset` |
| `RateLimiterStorage` / `WindowState` | Persist window snapshots |
| `RateLimit` | Accept / remaining tokens / `retry_after` |
| `RateLimiterError` | Invalid key / tokens / storage failures |

## Storage

| Store | Role |
| --- | --- |
| `InMemoryRateLimiterStorage` | Process-local (default DI) |

## App helpers

| Helper | Role |
| --- | --- |
| `require_accepted` | Turn a rejected `RateLimit` into `RateLimitExceeded` |
| `consume_or_exceed` | `consume` then require acceptance |
| `too_many_requests` | Build HTTP `429` with `Retry-After` and `X-RateLimit-*` |

## DI

`FrameworkExtension` adds [`RegisterDefaultRateLimiterPass`], which registers service id `rate_limiter.storage` (`DEFAULT_RATE_LIMITER_STORAGE_SERVICE`) tagged `rate_limiter.storage` with an `InMemoryRateLimiterStorage` when missing. Resolve `RateLimiterStorageService` and call `factory` / `fixed_window_factory` / `token_bucket_factory`.

## Example

```rust
use std::sync::Arc;
use std::time::Duration;
use serenade_rate_limiter::{
    InMemoryRateLimiterStorage, Policy, RateLimiterFactory, RateLimiterStorage,
    consume_or_exceed, too_many_requests,
};

let storage = Arc::new(InMemoryRateLimiterStorage::new());
let factory = RateLimiterFactory::new(
    "login",
    Policy::fixed_window(5, Duration::from_secs(15 * 60))?,
    storage,
)?;
let limiter = factory.create("alice")?;
match consume_or_exceed(&limiter, 1) {
    Ok(_) => { /* proceed */ }
    Err(serenade_rate_limiter::ConsumeOrExceedError::Exceeded(exceeded)) => {
        let _response = too_many_requests(&exceeded.rate_limit);
    }
    Err(other) => return Err(other.into()),
}
```

## Related

- Parent epic: [#186](https://github.com/Interchouette-ITC/Serenade/issues/186)
- Lock (critical sections): [LOCK.md](LOCK.md)
