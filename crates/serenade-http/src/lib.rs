//! HTTP foundation: request, response, attributes, and middleware contracts.
//!
//! Server adapters (Actix, Axum, and others) stay thin wrappers over this layer.

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// HTTP foundation marker until request/response types land.
pub const BOOTSTRAP: &str = "http-bootstrap";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_marker_is_non_empty() {
        assert_ne!(BOOTSTRAP, "");
    }
}
