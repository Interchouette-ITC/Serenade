# Bundles

Bundles are Serenade’s unit of **reusable composition**, analogous to Symfony bundles.

## What a bundle provides

| Artifact | Example |
| --- | --- |
| `DependencyInjection` extension | Register services, parameters |
| Config schema | `config/packages/serenade_foo.yaml` |
| Routes | `routes.yaml` or programmatic collection |
| Event subscribers | Tagged listeners |
| Console commands | `bin/console foo:bar` |
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
    ├── register_bundle (dependencies first)
    ├── compile (`Bundle::build`)
    ├── boot (`Bundle::boot`)
    └── shutdown (`Bundle::shutdown`, reverse order)
```

Route loading from bundles lands with the HTTP kernel. Bundle `CompilePass` registrations land with `serenade-di`.

## Bundle vs Wasm plugin

| | Bundle | Wasm / WIT plugin |
| --- | --- | --- |
| Trust | Same process, signed release | Sandboxed guest |
| Use | First-party features, trusted extensions | Merchant code, polyglot scripts |
| Serenade role | DI + lifecycle | Host capability bus (product) |

Both can coexist in RustaShop; Serenade bundles are the **in-process** story.

## Minimal bundle skeleton (future layout)

```text
crates/bundles/serenade-framework/
├── src/
│   ├── SerenadeFrameworkBundle.rs
│   ├── DependencyInjection/
│   └── Resources/config/services.yaml
└── Cargo.toml
```

Exact crate naming TBD when code starts; this doc locks the **model**, not the repo tree yet.
