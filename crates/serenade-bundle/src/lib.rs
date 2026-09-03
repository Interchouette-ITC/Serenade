//! Bundle composition unit for Serenade applications.
//!
//! Bundles register services, routes, and event subscribers during kernel boot.
//! The full `BundleInterface` registration surface is not implemented yet; this
//! crate currently exposes version and bootstrap markers only.

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Marker string confirming the bundle crate is linked.
pub const BOOTSTRAP: &str = "bundle-bootstrap";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert_ne!(version(), "");
    }
}
