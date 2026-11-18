# serenade-admin

Optional EasyAdmin-shaped admin CRUD helpers for Serenade HTML applications,
built on `serenade-form` (CSRF by default).

Opt in with `AdminBundle` / `AdminExtension`; it is not registered by
`FrameworkBundle`. Applications implement `AdminResourceHandler` for persistence.

Part of [Serenade](https://github.com/Interchouette-ITC/Serenade); most applications
depend on the [`serenade-framework`](https://crates.io/crates/serenade-framework) facade.
API documentation: <https://docs.rs/serenade-admin>.

Licensed under Apache-2.0.
