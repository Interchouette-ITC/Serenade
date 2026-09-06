# Observability (structured logging)

Monolog-like conventions live in **`serenade-observability`**. The preferred stack is
[`tracing`](https://docs.rs/tracing) with `tracing-subscriber` and `tracing-appender`.

| Piece | Role |
| --- | --- |
| `LoggingConfig` | Log directory, environment stem, levels, stderr/file sinks, rotation |
| `init` / `LoggingGuard` | Install the process subscriber; keep the guard until shutdown |
| Channel constants | Tracing `target` values (`serenade::app`, `serenade::request`, …) |

## App vs framework

| Owner | Responsibility |
| --- | --- |
| **Application** | Creates `var/log/` (or another path), chooses when to call `init`, ships `.gitignore` for logs |
| **Framework** | Documents defaults, provides `LoggingConfig::for_environment`, channel names, rolling file helper |

There is **no** mandatory global logger inside every Serenade crate. Components may emit `tracing` events when the app has installed a subscriber.

## Default layout

```text
{project}/var/log/dev.log     # Environment::Dev (daily rotation prefix)
{project}/var/log/prod.log
{project}/var/log/test.log
```

`LoggingConfig::for_environment(&env, project.join("var/log"))` sets:

- file stem `{env}.log`
- default level `DEBUG` in debug environments (`dev` / `test`), else `INFO`
- stderr **and** file enabled
- daily rotation via `tracing-appender`

Override filters with `SERENADE_LOG` (preferred) or `RUST_LOG` (same directive syntax as `EnvFilter`).

## Init hook

Call once near the start of `main`, after resolving `Environment` and before (or just after) kernel boot:

```rust
use serenade_kernel::Environment;
use serenade_observability::{init, LoggingConfig};

let environment = Environment::from_name("dev")?;
let log_dir = std::path::PathBuf::from("var/log");
let _logging = init(&LoggingConfig::for_environment(&environment, log_dir))?;

tracing::info!(target: serenade_observability::APP, "application starting");
```

Keep `_logging` (`LoggingGuard`) in scope until process exit so the non-blocking file writer flushes.

## Channels

| Constant | Target | Typical use |
| --- | --- | --- |
| `APP` | `serenade::app` | Domain / application messages |
| `REQUEST` | `serenade::request` | HTTP request lifecycle |
| `SECURITY` | `serenade::security` | AuthN/Z |
| `MESSENGER` | `serenade::messenger` | Bus dispatch |
| `KERNEL` | `serenade::kernel` | Boot / shutdown |

Filter example: `SERENADE_LOG=serenade::app=debug,serenade::security=info`.

Messenger apps can implement `serenade_messenger::LogSink` with `tracing::info!(target: serenade_observability::MESSENGER, …)`.

## Profiler

The Web Debug Toolbar / Profiler ([#56](https://github.com/Interchouette-ITC/Serenade/issues/56)) is a separate surface. Request-scoped log capture would subscribe to these channels; this crate does not install a profiler collector.

## Non-goals

- Full Monolog port or PHP handler matrix
- Mandating a second crates.io logging facade forever (`log` crate bridge is app-optional)
- ORM/SQL dump as the only surface (profiler DB collector)
