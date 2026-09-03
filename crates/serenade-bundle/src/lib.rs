//! Bundle composition helpers for Serenade applications.
//!
//! Provides [`Extension`], [`FrameworkBundle`], and [`build_container`] so apps
//! can wire router, config, and the event dispatcher the same way Symfony apps
//! load `FrameworkBundle`.

mod container;
mod error;
mod extension;
mod framework;

pub use container::build_container;
pub use error::BundleError;
pub use extension::Extension;
pub use framework::{
    FrameworkBundle, FrameworkExtension, CONFIG_SERVICE, FRAMEWORK_BUNDLE, ROUTER_SERVICE,
};
pub use serenade_kernel::{Bundle, BundleInterface, BundleRegistry};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
