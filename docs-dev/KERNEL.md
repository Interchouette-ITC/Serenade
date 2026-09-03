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
| **Configuration** | Layered config (defaults, env, bundle config files) |
| **Console** | CLI application, commands, scheduled task entry |
| **Cache** | PSR-like cache contracts; in-memory / Redis adapters |
| **Security** | AuthN/Z hooks, firewall abstraction, voter pattern |
| **Messenger** | Command/query/event bus; async transport adapters |
| **Serializer** | DTO ↔ JSON (and other formats); normalizers |
| **Validator** | Constraint validation on DTOs and commands |
| **Contracts** | Stable traits for persistence, clock, id generation, etc. |
| **Observability** | Tracing/metrics hooks; structured logging conventions |
| **Testing** | Kernel test harness, fake container, event assertion helpers |

## Lifecycle

```text
Created  →  register bundles (order is preserved)
         →  compile (`Bundle::build`)
         →  boot (`Bundle::boot`)
         →  shutdown (`Bundle::shutdown`, reverse order)
```

`serenade-kernel` exposes `Kernel`, `App`, and `Environment` (`dev` / `test` / `prod`). Debug defaults on for `dev` and `test`. An application with zero bundles is a valid boot.

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

## Request path (intent)

```text
HTTP adapter (Actix / Axum / …)
    → HTTP foundation (Request)
    → Middleware stack
    → Router
    → Controller / handler
    → Domain (application)
    → Event dispatch
    → Response
```

The **adapter** is thin. Serenade HTTP foundation and kernel stay server-agnostic.

## Messenger and jobs

| Pattern | Use |
| --- | --- |
| Command bus | Mutations with one handler |
| Query bus | Read models (optional CQRS) |
| Event bus | Side effects after commit |
| Async transport | Queue/worker integration (product chooses backend) |

RustaShop jobs (webhooks retry, agent runs, sandbox) should plug into messenger, not ad-hoc `spawn` everywhere.

## Configuration layers

```text
serenade.yaml (defaults)
    + config/packages/*.yaml (bundle defaults)
    + config/services.yaml (app services)
    + env-specific overrides
    + secrets (env / operator)
```

## Extension surface (kernel-level)

- Tagged services (event subscriber, command handler, …)
- Compiler passes / container extensions (bundle `build()` phase)
- Middleware ordering
- Route loaders from bundles

Domain-specific hooks (pricing, shipping) belong in **product bundles**, not the Serenade core.
