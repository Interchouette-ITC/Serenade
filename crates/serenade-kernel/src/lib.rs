//! Application kernel and lifecycle orchestration.
//!
//! Owns bundle registration order, environment boot, and graceful shutdown.
//! HTTP adapters and persistence stay in product crates.

mod application;
mod bundle;
mod environment;
mod error;
mod kernel;
mod registry;

pub use application::{App, Application};
pub use bundle::{Bundle, BundleInterface};
pub use environment::Environment;
pub use error::KernelError;
pub use kernel::{Kernel, KernelPhase};
pub use registry::BundleRegistry;

/// Compile-time crate version for diagnostics and health surfaces.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
