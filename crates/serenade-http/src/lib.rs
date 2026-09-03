//! HTTP foundation: request, response, attributes, and middleware contracts.
//!
//! Server adapters (Actix, Axum, and others) stay thin wrappers over this layer.
//! Request and response types are not implemented yet; this crate currently
//! exposes version and bootstrap markers only.

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Marker string confirming the HTTP foundation crate is linked.
pub const BOOTSTRAP: &str = "http-bootstrap";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_marker_is_non_empty() {
        assert_ne!(BOOTSTRAP, "");
    }
}
