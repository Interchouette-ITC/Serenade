//! Application kernel and lifecycle orchestration.
//!
//! Owns bundle registration order, environment boot, and graceful shutdown.
//! HTTP adapters and persistence stay in product crates.

/// Compile-time crate version for diagnostics and health surfaces.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Kernel integration marker until lifecycle APIs land in a later slice.
pub const BOOTSTRAP: &str = "kernel-bootstrap";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert_ne!(version(), "");
    }
}
