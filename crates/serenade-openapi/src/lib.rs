//! `OpenAPI` document helpers and optional explorer UI mounts for Serenade apps.
//!
//! Apps still declare their own `#[derive(OpenApi)]` paths and schemas. This crate
//! owns path conventions, Info/servers helpers, and (behind feature `actix`) mounting
//! Swagger UI, Redoc, `RapiDoc`, and Scalar beside a Serenade HTTP kernel.
//!
//! See `docs-dev/OPENAPI.md`.

mod config;
mod document;

#[cfg(feature = "actix")]
mod actix;

#[cfg(feature = "actix")]
pub use actix::configure_actix_ui;
pub use config::OpenApiUiPaths;
pub use document::{apply_info, apply_server, finalize_openapi};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_non_empty() {
        assert_ne!(super::version(), "");
    }
}
