//! Test harness helpers for Serenade applications and framework crates.
//!
//! Provides a [`SerenadeTestKernel`] that boots an [`App`](serenade_kernel::App)
//! in [`Environment::Test`](serenade_kernel::Environment::Test), an
//! [`HttpTestClient`] for foundation HTTP requests, and re-exports event
//! recording helpers from [`serenade_event`].

mod http_client;
mod kernel;

pub use http_client::HttpTestClient;
pub use kernel::SerenadeTestKernel;
pub use serenade_event::{assert_dispatched, RecordingSubscriber};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
