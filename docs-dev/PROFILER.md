# Web Debug Toolbar / Profiler

Serenade’s browser **profiler** is the analogue of Symfony’s Web Debug Toolbar + Profiler. It is **not** the console (`debug:container`, ratatui TUI).

| Surface | Role |
| --- | --- |
| Console (`serenade-console` / CLI) | Operator commands offline |
| Profiler (`serenade-profiler`) | Per-request browser debug in `dev` |

## Enablement

Disabled by default (`ProfilerConfig::disabled()`). Apps enable explicitly:

```rust
use std::sync::Arc;
use serenade_profiler::{AsyncProfilerMiddleware, ProfilerConfig, ProfileStore};

let store = Arc::new(ProfileStore::new(50));
let config = ProfilerConfig::enabled(50);
kernel.push_middleware(AsyncProfilerMiddleware::new(Arc::clone(&store), config));
```

Never leave the toolbar on by default in production configs.

## Request flow

1. Middleware allocates a token, stores it on the request as `_profiler_token`.
2. Controllers run; apps may call `record_query` for SQL panels.
3. On the way out, HTML responses gain a fixed toolbar linking to `/_profiler/{token}`.
4. `try_handle_profiler` serves the index and detail pages.

## Panels

| Panel | Source |
| --- | --- |
| Request / routing / timing | Middleware |
| Logs | `ProfilerLogLayer` + request scope (`with_profile_scope` / `install_log_scope`) |
| Database | App adapters → `record_query(store, token, QueryEvent)` |
| Views | Placeholder until a view layer exists |

## Observability bridge

Install `ProfilerLogLayer` on your `tracing` subscriber (alongside `serenade-observability` channels). While a profile scope is active, matching events appear on that request’s Logs panel.

## Console vs profiler

Documented also in `KERNEL.md` and `OBSERVABILITY.md`: CLI debug commands inspect the container and config; the profiler inspects **one HTTP request** in the browser.
