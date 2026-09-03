//! Layered configuration: defaults, bundle config, environment overrides.
//!
//! Secrets are read from the environment or operator-provided files, never hard-coded.

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Config integration marker until the loader API lands.
pub const BOOTSTRAP: &str = "config-bootstrap";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_workspace() {
        assert_eq!(version(), "0.1.0");
    }
}
