# HTTP ops helpers

Small helpers in `serenade-http` for request correlation, probes, and drain.
They stay framework-owned (Serenade `Middleware` / `AsyncMiddleware`), not a Tower stack.

## Request id

```rust
use serenade_http::{AsyncHttpKernel, AsyncRequestIdMiddleware, Request, Response, request_id};

let mut kernel = AsyncHttpKernel::from_sync(|request: &mut Request| {
    let _id = request_id(request);
    Ok(Response::text(200, "ok"))
});
kernel.push_middleware(AsyncRequestIdMiddleware);
```

- Header: `x-request-id` (inbound propagate, outbound echo)
- Attribute: `_serenade_request_id` (`String`)
- Empty inbound values are ignored; a new id is generated

Sync kernels use `RequestIdMiddleware` the same way.

## Health / readiness

```rust
use serenade_http::{HealthMiddleware, HttpKernel, Readiness, Request, Response};

let readiness = Readiness::new();
let mut kernel = HttpKernel::new(|_request: &mut Request| Ok(Response::text(200, "app")));
kernel.push_middleware(HealthMiddleware::new(readiness.clone()));
```

| Path | Meaning | Response |
| --- | --- | --- |
| `/healthz` | Liveness | `200` + `ok` |
| `/readyz` | Readiness | `200` + `ready`, or `503` + `not ready` |

Flip `readiness.mark_not_ready()` before draining traffic so load balancers stop sending new work.

Async: `AsyncHealthMiddleware`.

## Graceful shutdown recipe

1. `readiness.mark_not_ready()` (fail `/readyz`).
2. Stop the HTTP listener:
   - Actix: `bind_server` → `server.handle().stop(true).await`, then await the server
   - Axum: `bind_server` → `ShutdownHandle::shutdown()`, then `await_bound`
3. Run application `Kernel::shutdown()` (bundle teardown) after the listener is down.

The HTTP adapters do not call `Kernel::shutdown` themselves: that type lives in
`serenade-kernel` and stays optional at the adapter edge.
