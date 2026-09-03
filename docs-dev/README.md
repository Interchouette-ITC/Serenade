# Serenade - developer docs

English only. Workspace crates publish through normal PR flow to `dev`.

## Index

| Doc                          | Topic                                                  |
| ---------------------------- | ------------------------------------------------------ |
| [VISION.md](VISION.md)       | Why Symfony-shaped Rust; what Serenade is not          |
| [KERNEL.md](KERNEL.md)       | Kernel components and responsibilities                 |
| [BUNDLES.md](BUNDLES.md)     | Bundle model, extension points, composition            |
| [PERSISTENCE.md](PERSISTENCE.md) | Adapter pattern, repository traits, `UnitOfWork` |
| [RUSTASHOP.md](RUSTASHOP.md) | Illustrative RustaShop crate map (example, not locked) |

## Design rules

1. **Choice, not prescription** - persistence, ORM, HTTP server for the _application_ are application decisions. Serenade exposes contracts and adapters.
2. **Kernel vs product** - Serenade owns cross-cutting infrastructure; products own domain (commerce, CMS, etc.).
3. **Bundles compose components** - like Symfony bundles, not monolithic framework magic.
4. **Explicit extension points** - events, tagged services, middleware, WIT/sandbox hooks at the product layer when needed.

## Related product

[RustaShop](https://github.com/Interchouette-ITC/RustaShop) documents its own HTTP house pattern (Actix kernel + Axum MCP) in `docs-dev/FOUNDATIONS.md`. That is a **RustaShop** choice, not a Serenade requirement.

## Issues

| Epic | Focus |
| --- | --- |
| [#1](https://github.com/Interchouette-ITC/Serenade/issues/1) | Meta: framework foundations |
| [#2–#8](https://github.com/Interchouette-ITC/Serenade/issues/2) | Kernel, DI, events, HTTP, routing, config, console |
| [#9–#16](https://github.com/Interchouette-ITC/Serenade/issues/9) | Cache, security, messenger, serializer, validator, contracts, bundles, testing |
| [#17](https://github.com/Interchouette-ITC/Serenade/issues/17) | Cargo workspace and CI |
| [#18–#20](https://github.com/Interchouette-ITC/Serenade/issues/18) | Starter tasks (workspace, bundles, Actix adapter) |
| [#30](https://github.com/Interchouette-ITC/Serenade/issues/30) | Flex-like recipes and app scaffolding |

Config packages prefer **TOML**; console is the `bin/console` analogue (optional ratatui for rich TUI). Composer maps to **Cargo**, not a second package manager.

Consumer: [RustaShop #49](https://github.com/Interchouette-ITC/RustaShop/issues/49).
