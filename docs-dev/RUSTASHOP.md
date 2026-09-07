# RustaShop on Serenade (illustrative)

**Example only - not a locked RustaShop repo decision.** Shows how a Symfony-shaped product might sit on Serenade. Names match the live [rustashop](https://github.com/Interchouette-ITC/rustashop) tree where helpful.

## Layering

```text
RustaShop application
├── Serenade kernel (lifecycle, DI, events, HTTP foundation, …)
├── Serenade + RustaShop bundles
├── Chosen persistence (SQLx | SeaORM | Diesel spike)
├── Chosen HTTP adapters (Actix commerce API; Axum reserved for MCP)
└── UIs (Angular, rangular/Leptos) on OpenAPI + realtime
```

## Example crate map

```text
crates/
├── rustashop/                 # Application kernel (registers bundles)
├── rustashop-api/             # Commerce HTTP (Actix / Serenade listen)
├── rustashop-mcp/             # Axum MCP workspace member (no routes yet)
├── rustashop-domain/          # Pure domain (no ORM types in entities)
├── rustashop-persist/         # Feature-selected facade
├── rustashop-persist-sqlx/    # SQLx adapter
├── rustashop-persist-seaorm/  # SeaORM adapter
├── rustashop-persist-diesel/  # Diesel spike (not facade-wired)
├── rustashop-extensions/      # WIT Component Model host
└── rustashop-sandbox/         # Wasmer polyglot sandbox host
```

## Example domain modules (inside domain or bundles)

```text
rustashop-domain/
├── catalog/
├── cart/
├── checkout/
├── order/
├── customer/
├── payment/
├── shipping/
├── promotion/
└── tax/
```

Bundles then **compose** these modules: routes, services, subscribers, config.

## What Serenade gives RustaShop

| Need | Serenade |
| --- | --- |
| Service wiring | DI container |
| Checkout side effects | Event dispatcher + messenger |
| Admin + API auth | Security component |
| Config per env | Config component |
| CLI (migrate, seed, worker) | Console (#8; optional ratatui) |
| App scaffolding / recipes | Flex-like recipes (#30); Cargo for deps |
| DTO API ↔ JSON | Serializer + validator |

## What stays RustaShop-specific

- Money as integer minor units
- OpenAPI contract for both UIs
- WebSocket-first realtime ([RustaShop REALTIME.md](https://github.com/Interchouette-ITC/rustashop/blob/dev/docs-dev/REALTIME.md))
- WIT / Wasmer extension lanes
- AI-native tools and MCP ([RustaShop AI-NATIVE.md](https://github.com/Interchouette-ITC/rustashop/blob/dev/docs-dev/AI-NATIVE.md))

Framework work tracks in **this** repo; commerce epics track in the application repo.
