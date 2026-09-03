//! Layered configuration: defaults, bundle packages, environment interpolation.
//!
//! Secrets come from the environment or operator-provided files, never hard-coded.

mod document;
mod error;
mod packages;

pub use document::Config;
pub use error::ConfigError;
pub use packages::load_packages;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
