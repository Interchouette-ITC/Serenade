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

## Author checklist

Write a new first-party or third-party bundle in five steps:

1. **Struct + `BundleInterface`** - Implement `name()` (stable unique id). Optionally `dependencies()`, `build()`, `boot()`, `shutdown()`. Trait lives in `serenade-kernel` (re-exported from `serenade-bundle`).
2. **`Extension`** - Implement `alias()` (package config section) and `load(&Config, &mut ContainerBuilder)` to register services and parameters for that section.
3. **Package config** - Ship defaults under `config/packages/<alias>.toml` (YAML still accepted). Apps may overlay `config/packages/{env}/`.
4. **Optional contributions** - Implement `RouteLoader` for routes; tag event subscribers / console commands for the compile passes that `build_container` always runs.
5. **Register** - `App::register_bundle` (any order; kernel sorts by dependencies), then `build_container(..., &[&YourExtension, …])` after dotenv / environment are ready.

Worked example: [`examples/demo-app`](../examples/demo-app) (`DemoBundle` + `DemoExtension` + `config/packages/demo.toml`).

## Public vs private API

| Surface | Stability | Notes |
| --- | --- | --- |
| `BundleInterface` / `Bundle` | **Public** | Lifecycle + dependency names |
| `Extension` | **Public** | Package alias + `load` into DI |
| `build_container` | **Public** | Packages dir + env + extension list → `(Config, Container)` |
| `FrameworkBundle` / `FrameworkExtension` | **Public** | Empty `router` (`RouteCollection`); framework package alias |
| `BundleError` | **Public** | Extension / DI / config failures |
| `RouteLoader` (`serenade-http`) | **Public** | Optional route contribution |
| Service ids `config`, `router`, `event_dispatcher`, console application | **Public** | Documented constants / conventions |
| Compile-pass internals, private modules inside crates | **Private** | Do not depend on them from other crates |
| Undocumented `pub` items without rustdoc | Treat as **unstable** until documented |

SemVer for published Serenade crates: breaking changes to the public rows above require a major bump. Prefer depending on traits and documented service ids, not on concrete framework types behind feature flags you do not own.

## Third-party policy

| Rule | Detail |
| --- | --- |
| Stable contracts only | Depend on `BundleInterface`, `Extension`, `build_container`, and documented service ids. Do not call private kernel hooks or scrape undocumented modules. |
| Cargo crate layout | One Cargo package per bundle (or a small workspace). Export the bundle struct + extension; document the package alias and required config keys. |
| SemVer | Follow Cargo / SemVer for your crate. Pin Serenade majors carefully; test against the `dev` branch of the framework you target. |
| No private DI tricks | Do not reach into another bundle’s private service ids unless that bundle documents them as public. |
| Wasm is a different lane | In-process **bundles** are not the sandboxed Wasm / WIT host story. Host plumbing lives in `serenade-component-host` / `serenade-sandbox`; see [WASM.md](WASM.md). Product guests and ABIs are out of scope for this guide. |
| No marketplace requirement | Serenade does not ship a bundle registry. Discovery is Cargo / docs. |

## Core vs product bundles

| Class | Examples |
| --- | --- |
| **Serenade core bundles** | `FrameworkBundle`, and further framework bundles as they land |
| **Product bundles** | Application-owned feature bundles (catalog, cart, CMS, …) in the product repo |
| **Third-party** | Community Cargo crates that implement `BundleInterface` + `Extension` under this policy |

Symfony’s own features ship as bundles; product features should too, not as loose modules in one crate.

**Admin CRUD is not a core bundle.** There is no admin generator in `FrameworkBundle`. Apps own back-office UIs. Optional EasyAdmin-shaped helpers ship in **`serenade-admin`** (opt-in `AdminBundle` / `AdminExtension`, not auto-wired): [ADMIN.md](ADMIN.md) ([#177](https://github.com/Interchouette-ITC/Serenade/issues/177)).

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

`Extension` loads one package alias into a `ContainerBuilder`. `build_container` merges `config/packages/*.{toml,yaml,yml}`, then `config/packages/{env}/*` when that directory exists, applies flattened parameters, registers the root `config` service, runs each extension on `config.section(alias)`, and compiles with `RegisterEventSubscribersPass` and `RegisterCommandsPass` so `event_dispatcher` and console commands exist. Pass the environment name (same as `APP_ENV`) as the second argument.

`FrameworkBundle` / `FrameworkExtension` register the empty `router` (`RouteCollection`). Sample app: `examples/demo-app` (`DemoBundle` + `config/packages/*.toml`).

Bundles may also implement `RouteLoader` (`serenade-http`) to contribute routes.

## Bundle vs Wasm plugin

| | Bundle | Wasm / WIT guest |
| --- | --- | --- |
| Trust | Same process, signed release | Sandboxed guest |
| Use | First-party features, trusted extensions | Untrusted or polyglot guests |
| Serenade role | DI + lifecycle | Host **plumbing** (`serenade-component-host`, `serenade-sandbox`); product owns WIT ABI and fixtures |

Both can coexist in product apps; Serenade bundles are the **in-process** story. See [WASM.md](WASM.md).

## Layout today

```text
crates/serenade-bundle/     FrameworkBundle, Extension, build_container
examples/demo-app/          Canonical sample (DemoBundle)
├── config/packages/*.toml  Package config for alias `demo`
├── src/lib.rs + bundle.rs  BundleInterface, Extension, RouteLoader,
│                           demo:hello command, demo.ready subscriber
├── src/main.rs             Boot + dispatch demo.ready
└── src/bin/console.rs      Console entry with DemoBundle wired
```
