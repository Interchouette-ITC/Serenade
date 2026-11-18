# serenade-view

HTML view helpers for Serenade: reverse routing, assets, partials, and
engine-neutral document composition.

The optional `leptos-ssr` feature renders complete documents under a request-scoped
Leptos owner and injects managed metadata; `leptos-client` exposes the head root and
lifecycle components used by CSR and hydrated apps. Template engines such as Rangular
can target these components without becoming dependencies of this crate.

Part of [Serenade](https://github.com/Interchouette-ITC/Serenade); most applications
depend on the [`serenade-framework`](https://crates.io/crates/serenade-framework) facade.
API documentation: <https://docs.rs/serenade-view>.

Licensed under Apache-2.0.
