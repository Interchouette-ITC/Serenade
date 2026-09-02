# Serenade — developer docs

English only. Specification phase: no published crates yet.

## Index

| Doc | Topic |
| --- | --- |
| [VISION.md](VISION.md) | Why Symfony-shaped Rust; what Serenade is not |
| [KERNEL.md](KERNEL.md) | Kernel components and responsibilities |
| [BUNDLES.md](BUNDLES.md) | Bundle model, extension points, composition |
| [RUSTASHOP.md](RUSTASHOP.md) | Illustrative RustaShop crate map (example, not locked) |

## Design rules

1. **Choice, not prescription** — persistence, ORM, HTTP server for the *application* are application decisions. Serenade exposes contracts and adapters.
2. **Kernel vs product** — Serenade owns cross-cutting infrastructure; products own domain (commerce, CMS, etc.).
3. **Bundles compose components** — like Symfony bundles, not monolithic framework magic.
4. **Explicit extension points** — events, tagged services, middleware, WIT/sandbox hooks at the product layer when needed.

## Related product

[RustaShop](https://github.com/Interchouette-ITC/RustaShop) documents its own HTTP house pattern (Actix kernel + Axum MCP) in `docs-dev/FOUNDATIONS.md`. That is a **RustaShop** choice, not a Serenade requirement.
