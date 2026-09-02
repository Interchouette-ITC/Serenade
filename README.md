# Serenade

Symfony-oriented application framework for Rust. **Not** Rails-style, **not** Django-style: composable kernel components and bundles around which you build products.

Serenade does **not** impose SQLx, SeaORM, Diesel, rangular, Leptos, or Actix vs Axum for your product HTTP surface. It provides architecture, contracts, and reusable components; the application chooses persistence and UI stacks.

**Primary consumer (in progress):** [RustaShop](https://github.com/Interchouette-ITC/RustaShop) — commerce kernel on Serenade, the way PrestaShop builds on Symfony.

**Status:** specification and Cursor nest. No stable crates yet.

## Docs

Implementers start here:

- [`docs-dev/README.md`](docs-dev/README.md) — index
- [`docs-dev/VISION.md`](docs-dev/VISION.md) — Symfony analogy, non-goals
- [`docs-dev/KERNEL.md`](docs-dev/KERNEL.md) — kernel components
- [`docs-dev/BUNDLES.md`](docs-dev/BUNDLES.md) — bundles and extension model
- [`docs-dev/RUSTASHOP.md`](docs-dev/RUSTASHOP.md) — example application layout (illustrative)

## Relationship to RustaShop

| Layer | Role |
| --- | --- |
| **Serenade** | Kernel lifecycle, DI, events, HTTP foundation, routing, config, console, cache, security, messenger, serializer, validator, contracts, bundles |
| **RustaShop** | Commerce domain (catalog, cart, checkout, …) composed from Serenade + chosen persistence and UIs |

Serenade is the reusable framework repo. RustaShop is the product repo.

## License

To be decided before first crate publish. Interchouette-ITC projects often use **Apache-2.0**.
