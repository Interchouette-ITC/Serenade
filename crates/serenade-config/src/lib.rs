//! Layered configuration: dotenv, bundle packages, env overlays, interpolation.
//!
//! Secrets come from the environment or operator-provided files, never hard-coded.

mod document;
mod dotenv;
mod error;
mod packages;

pub use document::Config;
pub use dotenv::load_dotenv;
pub use error::ConfigError;
pub use packages::{load_packages, load_packages_for_env};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
