//! Structured logging for Serenade apps (Monolog-like conventions on `tracing`).
//!
//! Apps own the log directory. Call [`init`] early in `main` (before kernel boot).
//! Optional OpenTelemetry export is available behind feature `otel`.
//! See `docs-dev/OBSERVABILITY.md`.

mod channels;
mod config;
mod error;
mod init;
#[cfg(feature = "otel")]
mod otel;

pub use channels::{APP, KERNEL, MESSENGER, REQUEST, SECURITY};
pub use config::{LoggingConfig, Rotation};
pub use error::ObservabilityError;
pub use init::{LoggingGuard, build_subscriber, init};
#[cfg(feature = "otel")]
pub use otel::{OtelConfig, OtelGuard, build_subscriber_with_otel, init_with_otel};

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
