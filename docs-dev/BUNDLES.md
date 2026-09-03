# Bundles

Bundles are Serenade’s unit of **reusable composition**, analogous to Symfony bundles.

## What a bundle provides

| Artifact | Example |
| --- | --- |
| `DependencyInjection` extension | Register services, parameters |
| Config schema | `config/packages/serenade_foo.toml` (YAML still accepted) |
| Routes | `routes.toml` / `routes.yaml` or programmatic collection |
| Event subscribers | Tagged listeners |
| Console commands | `bin/console foo:bar` (Serenade console Application) |
| Public services | Exported interfaces for other bundles |

## Core vs product bundles

| Class | Examples |
| --- | --- |
| **Serenade core bundles** | `FrameworkBundle`, `SecurityBundle`, `ConsoleBundle`, `MessengerBundle` |
| **RustaShop bundles** (product) | `CatalogBundle`, `CartBundle`, `CheckoutBundle`, `PaymentBundle` |
| **Third-party / merchant** | Extension bundles or WIT plugins (product policy) |

Symfony’s own features ship as bundles; RustaShop commerce features should too, not as loose modules in one crate.

## Registration

```text
Kernel / App
    ├── register_bundle (any order)
    ├── compile (topological sort by BundleInterface::dependencies, then build)
    ├── boot (`BundleInterface::boot`)
    └── shutdown (`BundleInterface::shutdown`, reverse dependency order)
```

`BundleInterface` (alias `Bundle`) declares `name`, optional `dependencies`, then `build` / `boot` / `shutdown`. `BundleRegistry` sorts dependents after their dependencies; unknown deps and cycles fail at compile.

## Extensions and package config

`Extension` loads one package alias into a `ContainerBuilder`. `build_container` merges `config/packages/*.{toml,yaml,yml}`, then `config/packages/{env}/*` when that directory exists, applies flattened parameters, registers the root `config` service, runs each extension on `config.section(alias)`, and compiles with `RegisterEventSubscribersPass` so `event_dispatcher` exists. Pass the environment name (same as `APP_ENV`) as the second argument.

`FrameworkBundle` / `FrameworkExtension` register the empty `router` (`RouteCollection`). Sample app: `examples/demo-app` (`DemoBundle` + `config/packages/*.toml`).

Bundles may also implement `RouteLoader` (`serenade-http`) to contribute routes.

## Bundle vs Wasm plugin

| | Bundle | Wasm / WIT plugin |
| --- | --- | --- |
| Trust | Same process, signed release | Sandboxed guest |
| Use | First-party features, trusted extensions | Merchant code, polyglot scripts |
| Serenade role | DI + lifecycle | Host capability bus (product) |

Both can coexist in product apps; Serenade bundles are the **in-process** story.

## Layout today

```text
crates/serenade-bundle/     FrameworkBundle, Extension, build_container
examples/demo-app/
├── config/packages/*.toml
└── src/main.rs             DemoBundle + FrameworkBundle
```
