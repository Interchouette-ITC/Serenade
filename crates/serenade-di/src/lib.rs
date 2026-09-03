//! Dependency injection container.
//!
//! Symfony-style service wiring without mandating a specific HTTP or ORM stack.

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// DI integration marker until the container API lands.
pub const BOOTSTRAP: &str = "di-bootstrap";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_marker_is_stable() {
        assert_eq!(BOOTSTRAP, "di-bootstrap");
    }
}
