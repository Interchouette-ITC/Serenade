# RustaShop on Serenade (illustrative)

**Example only - not a locked RustaShop repo decision.** Shows how a Symfony-shaped product might sit on Serenade.

## Layering

```text
RustaShop application
├── Serenade kernel (lifecycle, DI, events, HTTP foundation, …)
├── Serenade + RustaShop bundles
├── Chosen persistence (SQLx | SeaORM | Diesel)
├── Chosen HTTP adapters (Actix kernel API, Axum MCP - current RustaShop doc)
└── UIs (Angular, rangular/Leptos) on OpenAPI + realtime
```

## Example crate map

```text
crates/
├── rustashop/              # Application kernel (registers bundles)
├── rustashop-api/          # HTTP adapter: Actix (product choice today)
├── rustashop-mcp/          # HTTP adapter: Axum MCP sidecar
├── rustashop-domain/       # Pure domain (no Serenade imports in entities)
├── rustashop-persistence/  # SQLx OR SeaORM OR Diesel - app choice
├── rustashop-events/       # Domain events + Serenade dispatcher wiring
├── rustashop-jobs/         # Messenger transports / workers
└── rustashop-extension/    # Extension host glue (WIT, sandbox bridges)
```

## Example domain modules (inside domain or bundles)

```text
rustashop-domain/   (or bundle-per-boundary later)
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
- WebSocket-first realtime ([RustaShop REALTIME.md](https://github.com/Interchouette-ITC/RustaShop/blob/dev/docs-dev/REALTIME.md))
- WIT / Wasmer extension lanes
- AI-native tools and MCP ([RustaShop AI-NATIVE.md](https://github.com/Interchouette-ITC/RustaShop/blob/dev/docs-dev/AI-NATIVE.md))

## Delivery order (suggested)

1. Serenade: kernel boot + DI + config + event dispatcher (minimal)
2. Serenade: HTTP foundation + one adapter (Actix or Axum)
3. RustaShop: domain types + persistence adapter choice
4. RustaShop: `CatalogBundle` + read API
5. Expand bundles along MVP epics in RustaShop GitHub

Track framework work in **this** repo; track commerce epics in **RustaShop**.
