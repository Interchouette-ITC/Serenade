//! Bundle composition helpers for Serenade applications.
//!
//! The registration contract lives on [`serenade_kernel::BundleInterface`].
//! This crate re-exports that surface for product bundles; core wiring such as
//! `FrameworkBundle` lands in follow-up crates on top of the same traits.

pub use serenade_kernel::{Bundle, BundleInterface, BundleRegistry};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert_ne!(version(), "");
    }

    #[test]
    fn registry_sorts_by_dependencies() {
        struct Framework;
        struct App;

        impl BundleInterface for Framework {
            fn name(&self) -> &'static str {
                "framework"
            }
        }

        impl BundleInterface for App {
            fn name(&self) -> &'static str {
                "app"
            }

            fn dependencies(&self) -> &'static [&'static str] {
                &["framework"]
            }
        }

        let mut registry = BundleRegistry::new();
        registry.register(App).expect("app");
        registry.register(Framework).expect("framework");
        let sorted = registry.sorted().expect("sort");
        let names: Vec<_> = sorted.iter().map(|bundle| bundle.name()).collect();
        assert_eq!(names, ["framework", "app"]);
    }
}
