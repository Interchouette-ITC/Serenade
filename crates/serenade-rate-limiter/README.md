# serenade-rate-limiter

Symfony-shaped rate limiting: `Policy` (token bucket / fixed window),
`RateLimiterFactory` / `RateLimiter`, `RateLimiterStorage`,
`InMemoryRateLimiterStorage`, app helpers (`consume_or_exceed`),
HTTP `too_many_requests`, and DI via `RegisterDefaultRateLimiterPass`
(service id `rate_limiter.storage`).

See `docs-dev/RATE_LIMITER.md`.
