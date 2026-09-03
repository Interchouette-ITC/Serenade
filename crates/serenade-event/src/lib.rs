//! Event dispatcher for domain and infrastructure events.
//!
//! Subscribers are registered by bundles; dispatch stays synchronous by default.

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Event integration marker until the dispatcher API lands.
pub const BOOTSTRAP: &str = "event-bootstrap";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver_shape() {
        assert!(version().contains('.'));
    }
}
