# Kernel components

Cross-cutting infrastructure Serenade owns. Products register domain services and bundles on top.

## Component map

| Component | Responsibility |
| --- | --- |
| **Kernel / lifecycle** | Boot, environment, bundle registration order, shutdown |
| **Dependency injection** | Service container, autowiring-style resolution, scoped services |
| **Event dispatcher** | Sync domain and infrastructure events; subscriber tags |
| **HTTP foundation** | Request/response types, attributes, lifecycle (framework-agnostic core) |
| **HTTP kernel** | Middleware pipeline, controller/action resolution |
| **Routing** | Route collection, requirements, method constraints |
| **Configuration** | Layered config (defaults, env, TOML package files; YAML still accepted) |
| **Console** | CLI application (`bin/console` analogue), commands, optional rich TUI |
| **Cache** | PSR-like cache contracts; in-memory / Redis adapters |
| **Security** | AuthN/Z hooks, firewall abstraction, voter pattern |
| **Messenger** | Command/query/event bus; async transport adapters |
| **Serializer** | DTO ↔ JSON (and other formats); normalizers |
| **Validator** | Constraint validation on DTOs and commands |
| **Contracts** | Stable traits for persistence, clock, id generation, etc. |
| **Observability** | Structured logging conventions (Monolog-like channels / `var/log` layout); tracing/metrics hooks; profiler bridge later |
| **Testing** | Kernel test harness, fake container, event assertion helpers |

## Lifecycle

```text
Created  →  register bundles (any order)
         →  compile (sort by dependencies, then `BundleInterface::build`)
         →  boot (`BundleInterface::boot`)
         →  shutdown (`BundleInterface::shutdown`, reverse dependency order)
```

`serenade-kernel` exposes `Kernel`, `App`, `BundleInterface` / `Bundle`, `BundleRegistry`, and `Environment` (`dev` / `test` / `prod`, plus `Custom` for names like `staging` or `recette`). Debug defaults on for `dev` and `test` only. An application with zero bundles is a valid boot.

`Kernel::boot` compiles first when the kernel is still in `Created`. After `Shutdown` the kernel is terminal.

## Dependency injection

`serenade-di` provides `ContainerBuilder` → compile passes → `Container`.

| Concept | Role |
| --- | --- |
| `ServiceDefinition` | Id, `Scope` (singleton / prototype), declared `Reference` dependencies |
| `ParameterBag` | String parameters available to factories |
| `CompilePass` | Extensible pipeline run before the container freezes |
| `Container::get` / `get_as` | Resolve by id or alias; detects circular dependency at compile and resolve |

Factories return `Box<dyn Any + Send + Sync>`; callers downcast with `get_as`.

## Events

`serenade-event` provides a synchronous `EventDispatcher`.

| Concept | Role |
| --- | --- |
| `Event` | Named payload (`cart.updated`, `order.placed`, …) |
| `EventSubscriber` | Handles one event name; higher `priority` runs first |
| `RegisterEventSubscribersPass` | Collects services tagged `event.subscriber` into `event_dispatcher` |
| `RecordingSubscriber` / `assert_dispatched` | Test harness for dispatch order |

Dispatch stays in-process. Async transports belong on the messenger component.

## Request path

`serenade-http` owns framework-agnostic types. Thin server adapters copy bytes in and out.

```text
HTTP adapter (Actix / Axum / …)
    → Request (method, path, headers, body, attributes)
    → UrlMatcher (RouteCollection; writes `_route` + path params)
    → Middleware stack (first registered layer is outermost)
    → Controller (`RequestHandler`)
    → Response
```

`HttpKernel::handle` always returns a `Response`. Handler and middleware errors go through `ExceptionHandler` (`DefaultExceptionHandler` maps `HttpError` status and message to `text/plain`).

`AsyncHttpKernel` is the async counterpart: controllers return a future (database I/O). Wrap sync handlers with `AsyncHttpKernel::from_sync`. Actix `listen` / `app` take `AsyncHttpKernel`; sync `dispatch` remains for `HttpKernel`.

Routing lives in `serenade-http`: `Route` / `RouteCollection`, `UrlMatcher` (404 / 405), and `RouteLoader` for bundles. Path segments `{name}` become request attributes.

## HTTP adapters

Server crates stay thin:

1. Map the server request (method, path, headers, body) into `serenade_http::Request`.
2. Optionally run `UrlMatcher::apply` to set `_route` and path parameters.
3. Call `HttpKernel::handle`.
4. Map `serenade_http::Response` back to the server response type.

`serenade-http-actix` implements that bridge for Actix Web (`from_actix`, `to_actix`, `dispatch` / `dispatch_async`). For skeletons that only need to bind and serve a kernel, call `serenade_http_actix::listen(addr, async_kernel)` (or `app(data)` when composing Actix yourself). An Axum adapter can reuse the same four steps without changing the foundation crate.

## Messenger and jobs

| Pattern | Use |
| --- | --- |
| Command bus | Mutations with one handler |
| Query bus | Read models (optional CQRS) |
| Event bus | Side effects after commit |
| Async transport | Queue/worker integration (product chooses backend) |

RustaShop jobs (webhooks retry, agent runs, sandbox) should plug into messenger, not ad-hoc `spawn` everywhere.

## Configuration layers

`serenade-config` loads **TOML** and YAML mappings, deep-merges them, interpolates `${VAR}` / `${VAR:-default}`, and flattens dotted keys into the DI `ParameterBag`.

**Preference:** new apps and Serenade examples use `config/packages/*.toml`. YAML remains supported for Symfony familiarity. There is no PHP/XML config track.

```text
load_dotenv(project_root, APP_ENV)
    .env → .env.local (not in prod) → .env.{env} → .env.{env}.local
    (later files override earlier; process env never overwritten)
defaults (TOML or YAML)
    + config/packages/*.{toml,yaml,yml}  (sorted by file name; files only)
    + config/packages/{env}/*.{toml,yaml,yml}  (env overlay when the directory exists)
    + ${VAR} interpolation from the process environment
```

`load_packages` loads the base directory only. `load_packages_for_env` applies the overlay. `build_container(packages_dir, environment, extensions)` uses the env-aware loader. Apps should call `load_dotenv` on the project root before building the container when they ship `.env` files.

Mappings merge; scalars and sequences in an overlay replace the base value. Unset variables without a default fail load.

## Console

The console component is Serenade’s `bin/console` analogue: discoverable commands, `--env` / `--no-debug`, kernel boot for ops (migrate, workers, debug). Plain commands use clap; interactive debug surfaces may use **ratatui**. See [CONSOLE.md](CONSOLE.md). Flex-like app scaffolding is separate (#30); Cargo remains the package manager.

## Extension surface (kernel-level)

- Tagged services (event subscriber, command handler, …)
- Compiler passes / container extensions (bundle `build()` phase)
- Middleware ordering
- Route loaders from bundles

Domain-specific hooks (pricing, shipping) belong in **product bundles**, not the Serenade core.
