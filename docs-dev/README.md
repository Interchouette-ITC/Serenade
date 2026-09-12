# Serenade - developer docs

English only. Workspace crates publish through normal PR flow to `dev`.

## Index

| Doc                          | Topic                                                  |
| ---------------------------- | ------------------------------------------------------ |
| [VISION.md](VISION.md)       | Why Symfony-shaped Rust; what Serenade is not (incl. no core admin generator) |
| [KERNEL.md](KERNEL.md)       | Kernel components; admin / back-office ownership |
| [CONSOLE.md](CONSOLE.md)     | Console Application, commands, `--env` / ratatui       |
| [RECIPES.md](RECIPES.md)     | Flex-like recipes, `serenade new`, Cargo stays PM      |
| [SECURITY.md](SECURITY.md)   | AuthN/Z, firewall, CSRF, password hashing, session login |
| [FORMS.md](FORMS.md)         | HTML forms, CSRF default-on, XSS-safe render   |
| [I18N.md](I18N.md)           | Translator, catalogues, locale, ICU helpers    |
| [PROFILER.md](PROFILER.md)   | Web debug toolbar / per-request profiler       |
| [VIEW.md](VIEW.md)           | path / asset / partial HTML helpers            |
| [SERIALIZER.md](SERIALIZER.md) | JSON + optional TOON (agent context export)  |
| [MYFEED.md](MYFEED.md)       | Beginner demo: open public feed (`examples/MyFeed`) |
| [CACHE.md](CACHE.md)         | Cache pools, ArrayAdapter, RedisAdapter (redis-rs)     |
| [SESSION.md](SESSION.md)     | Session bag, flash, SessionStore, cookie + HTTP middleware |
| [SEARCH.md](SEARCH.md)       | Document index contracts, MemorySearchAdapter          |
| [MAILER.md](MAILER.md)       | Email + Mime lite, Null/File/SMTP transports, DI       |
| [OBSERVABILITY.md](OBSERVABILITY.md) | Tracing init, channels, `var/log` file sinks    |
| [ERRORS.md](ERRORS.md)       | `thiserror` / Display / Debug / API error habits       |
| [BUNDLES.md](BUNDLES.md)     | Bundle model; admin CRUD is not FrameworkBundle        |
| [ADMIN.md](ADMIN.md)         | Optional `serenade-admin` CRUD (list/show; forms next) |
| [WASM.md](WASM.md)           | Wasm host plumbing (wasmtime CM + Wasmer WASIX)        |
| [PERSISTENCE.md](PERSISTENCE.md) | Adapter pattern, repository traits, `UnitOfWork` |
| [RUSTASHOP.md](RUSTASHOP.md) | Illustrative RustaShop crate map (example, not locked) |

## Design rules

1. **Choice, not prescription** - persistence, ORM, HTTP server for the _application_ are application decisions. Serenade exposes contracts and adapters.
2. **Kernel vs product** - Serenade owns cross-cutting infrastructure; products own domain (commerce, CMS, etc.).
3. **Bundles compose components** - like Symfony bundles, not monolithic framework magic.
4. **Explicit extension points** - events, tagged services, middleware; Wasm host **plumbing** in Serenade ([WASM.md](WASM.md)), WIT/sandbox **ABI and fixtures** at the product layer.

## Related product

[RustaShop](https://github.com/Interchouette-ITC/RustaShop) documents its own HTTP house pattern (Actix kernel + Axum MCP) in `docs-dev/FOUNDATIONS.md`. That is a **RustaShop** choice, not a Serenade requirement.

## Issues

| Epic | Focus |
| --- | --- |
| [#1](https://github.com/Interchouette-ITC/Serenade/issues/1) | Meta: framework foundations |
| [#2-#8](https://github.com/Interchouette-ITC/Serenade/issues/2) | Kernel, DI, events, HTTP, routing, config, console |
| [#9-#16](https://github.com/Interchouette-ITC/Serenade/issues/9) | Cache, security, messenger, serializer, validator, contracts, bundles, testing |
| [#17](https://github.com/Interchouette-ITC/Serenade/issues/17) | Cargo workspace and CI |
| [#18-#20](https://github.com/Interchouette-ITC/Serenade/issues/18) | Starter tasks (workspace, bundles, Actix adapter) |
| [#30](https://github.com/Interchouette-ITC/Serenade/issues/30) | Flex-like recipes and app scaffolding |
| [#56](https://github.com/Interchouette-ITC/Serenade/issues/56) | Web Debug Toolbar / Profiler (logs + DB/query panels) |
| [#57](https://github.com/Interchouette-ITC/Serenade/issues/57) | Observability / structured logging (Monolog-like) |
| [#110](https://github.com/Interchouette-ITC/Serenade/issues/110) | Forms, CSRF, HTML escape |
| [#111](https://github.com/Interchouette-ITC/Serenade/issues/111) | View helpers (`path` / `asset` / partials) |
| [#122](https://github.com/Interchouette-ITC/Serenade/issues/122) | Admin generator stance (not in kernel / FrameworkBundle) |
| [#177](https://github.com/Interchouette-ITC/Serenade/issues/177) | Implement optional `serenade-admin` CRUD bundle |
| [#123](https://github.com/Interchouette-ITC/Serenade/issues/123) | Docs: lock admin generator as core non-goal |
| [#124](https://github.com/Interchouette-ITC/Serenade/issues/124) | Parked: optional Admin CRUD bundle (EasyAdmin-shaped) |

Config packages prefer **TOML**; console is the `bin/console` analogue (optional ratatui for rich TUI). Composer maps to **Cargo**, not a second package manager.

Consumer: [RustaShop #49](https://github.com/Interchouette-ITC/RustaShop/issues/49).
