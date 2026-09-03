//! Stable contracts for adapters implemented by applications.
//!
//! Repository and unit-of-work traits live here with zero database dependencies.
//! Commerce domain types belong in product crates, not Serenade.

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Contracts integration marker until repository traits land (#22).
pub const BOOTSTRAP: &str = "contracts-bootstrap";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contracts_crate_is_framework_only() {
        assert_eq!(BOOTSTRAP, "contracts-bootstrap");
    }
}
