# Web Debug Toolbar / Profiler

Serenade’s browser **profiler** is the analogue of Symfony’s Web Debug Toolbar + Profiler. It is **not** the console (`debug:container`, ratatui TUI).

| Surface                            | Role                               |
| ---------------------------------- | ---------------------------------- |
| Console (`serenade-console` / CLI) | Operator commands offline          |
| Profiler (`serenade-profiler`)     | Per-request browser debug in `dev` |

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
2. Controllers run; apps may call `record_query`, `record_debug`, and `record_view`.
3. On the way out, HTML responses gain a fixed toolbar linking to `/_profiler/{token}`.
4. `try_handle_profiler` serves the index and detail pages.

## Panels

| Panel                      | Source                                                                          |
| -------------------------- | ------------------------------------------------------------------------------- |
| Request / routing / timing | Middleware                                                                      |
| Logs                       | `ProfilerLogLayer` + request scope (`with_profile_scope` / `install_log_scope`) |
| Database                   | App adapters → `record_query(store, token, QueryEvent)`                         |
| Debug                      | App → `record_debug(store, token, label, value)`                                |
| Views                      | App → `record_view(store, token, name, duration)`                               |

## Symfony map (DebugBundle / VarDumper)

| Symfony                         | Serenade                                                                          |
| ------------------------------- | --------------------------------------------------------------------------------- |
| `DebugBundle` (dev dumps)       | `record_debug` + Debug panel (dev-only via `ProfilerConfig`)                      |
| `dump()` / VarDumper            | Plain labeled strings on the Debug panel (no object-graph walker, no dump server) |
| Twig / template timing panel    | `record_view` + Views panel (apps name their own views)                           |
| Web Profiler toolbar            | `inject_toolbar` + `/_profiler/{token}`                                           |

Keep dumps out of production: leave the profiler disabled outside local/dev configs.

## Performance tooling

Request-scoped panels above are for **dev inspection**. Optional app-level hotpath / micro-benchmark recipes live in `PERF.md` when that guide lands; they complement the profiler and do not replace it.

## Observability bridge

Install `ProfilerLogLayer` on your `tracing` subscriber (alongside `serenade-observability` channels). While a profile scope is active, matching events appear on that request’s Logs panel.

## Console vs profiler

Documented also in `KERNEL.md` and `OBSERVABILITY.md`: CLI debug commands inspect the container and config; the profiler inspects **one HTTP request** in the browser.
