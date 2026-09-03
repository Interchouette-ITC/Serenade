//! Bundle composition unit for Serenade applications.
//!
//! Bundles register services, routes, and event subscribers during kernel boot.

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Bundle integration marker until `BundleInterface` lands.
pub const BOOTSTRAP: &str = "bundle-bootstrap";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!version().is_empty());
    }
}
