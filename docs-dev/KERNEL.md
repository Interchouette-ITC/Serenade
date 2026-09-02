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
