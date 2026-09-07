//! Structured logging for Serenade apps (Monolog-like conventions on `tracing`).
//!
//! Apps own the log directory. Call [`init`] early in `main` (before kernel boot).
//! See `docs-dev/OBSERVABILITY.md`.

mod channels;
mod config;
mod error;
mod init;

pub use channels::{APP, KERNEL, MESSENGER, REQUEST, SECURITY};
pub use config::{LoggingConfig, Rotation};
pub use error::ObservabilityError;
pub use init::{LoggingGuard, build_subscriber, init};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_non_empty() {
        assert_ne!(super::version(), "");
    }
}
