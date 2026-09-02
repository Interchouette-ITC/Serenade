# Vision

Serenade is a **Symfony-oriented** Rust application framework: kernel, components, and bundles. Products assemble them; Serenade does not ship a full vertical app.

## What we copy from Symfony (conceptually)

```text
Serenade
├── Kernel / application lifecycle
├── Dependency injection
├── Event dispatcher
├── HTTP foundation / request lifecycle
├── Routing
├── Configuration
├── Console
├── Cache
├── Security
├── Messenger / async jobs
├── Serializer
├── Validator
├── Contracts (interfaces)
├── Extensibility / bundles
└── Independent components (usable without full stack)
```

Symfony organizes its own features as bundles (`FrameworkBundle`, `SecurityBundle`, `DebugBundle`, …). Serenade follows the same **composition** idea: bundles register services, routes, config, and event subscribers into the kernel.

## What Serenade is not

| Not | Why |
| --- | --- |
| Rails / Django “batteries included” app server | No opinionated ORM + admin + templates in one box |
| A commerce product | That is RustaShop (or other apps) |
| A UI framework | Angular, rangular/Leptos, etc. stay in the application |
| A single HTTP crate mandate | Actix, Axum, or Tower-only apps integrate via HTTP foundation adapters |

## Persistence and HTTP: application choice

Like Symfony with Doctrine **or** DBAL **or** custom storage:

```text
Application persistence (pick one or adapter)
├── SQLx
├── SeaORM
├── Diesel
└── …

Application HTTP server (pick one)
├── Actix-web
├── Axum
└── …
```

Serenade defines **contracts** (repository traits, unit of work hooks, request/response abstractions) and optional bridge crates. It does not force one database layer.

## PrestaShop / Symfony parallel

| PrestaShop / Symfony | Serenade / RustaShop |
| --- | --- |
| Symfony kernel + components | Serenade kernel + components |
| Symfony bundles | Serenade bundles + RustaShop bundles |
| PrestaShop domain (catalog, cart, …) | RustaShop domain modules |
| Merchant modules / hooks | Product extension story (WIT, events, bundles) |

RustaShop should feel “ Symfony-backed ” in **architecture**, not in PHP or in copying Symfony APIs literally.

## Non-goals (early)

- 1:1 Symfony PHP API port
- Shipping every Symfony component before kernel + DI + events + HTTP foundation exist
- Replacing RustaShop `docs-dev` commerce foundations
