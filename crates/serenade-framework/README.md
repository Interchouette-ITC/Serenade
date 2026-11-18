# serenade-framework

Serenade is a Symfony-oriented application framework for Rust: a kernel with
bundles, a dependency-injection container, an event dispatcher, layered
configuration, and optional components (HTTP, security, messaging, views,
forms, ...) that applications compose.

`serenade-framework` is the public entry point for Serenade applications. It
re-exports a curated surface of the independently published `serenade-*`
crates behind Cargo features.

```toml
[dependencies]
serenade-framework = { version = "1.0.0", features = ["http", "validation"] }
```

The default build contains only the foundations (`kernel`, `config`, `di`,
`event`) and needs no web server, database, Leptos, or Rangular.

| Feature | Enables |
| --- | --- |
| `console` | `serenade-console` |
| `bundle` | `serenade-bundle` |
| `http` | `serenade-http`, `serenade-http-client` |
| `http-actix` / `http-axum` | Actix-web / Axum adapters |
| `http-client-reqwest` | Reqwest outbound client |
| `session`, `security`, `validation` | Session, security, and validator components |
| `security-oauth` / `security-ldap` | OAuth / LDAP helpers |
| `lock`, `rate-limiter`, `scheduler`, `messenger` | Lock, rate limiter, scheduler, message bus |
| `scheduler-messenger` | Scheduler dispatch through the message bus |
| `mailer` / `mailer-smtp` / `mailer-esp` | Mailer and its SMTP / ESP transports |
| `notifier` | SMS and push notifications |
| `view` / `view-leptos-ssr` / `view-leptos-client` | Views and Leptos SSR / hydration support |
| `form` | `view` + CSRF-protected HTML forms |

`bundle` does not enable reqwest or SMTP, including when combined with `http`
or `mailer`. Enable `http-client-reqwest` or `mailer-smtp` explicitly.

```rust
use serenade_framework::prelude::*;

struct Shop;

impl BundleInterface for Shop {
    fn name(&self) -> &'static str {
        "shop"
    }
}

let mut app = App::new(Environment::Prod);
app.register_bundle(Shop)?;
app.boot()?;
```

The `serenade` command-line tool is shipped by the separate
[`serenade-cli`](https://crates.io/crates/serenade-cli) package
(`cargo install serenade-cli`). The crates.io package named `serenade` is
unrelated to this project.

- Website: <https://serenade.rs>
- Guides: <https://docs.serenade.rs>
- API documentation: <https://docs.rs/serenade-framework>
- Repository: <https://github.com/Interchouette-ITC/Serenade>
- Minimum supported Rust version: 1.100

Licensed under Apache-2.0.
