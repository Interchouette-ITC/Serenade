//! Wasmtime Component Model host helpers.
//!
//! Serenade owns engine plumbing. Product apps own WIT worlds, `bindgen!`, and
//! guest ABIs. `FrameworkBundle` does not register this crate.
//!
//! Enable Cargo feature `wasmtime` (default).

#![forbid(unsafe_code)]

#[cfg(feature = "wasmtime")]
mod host;

#[cfg(feature = "wasmtime")]
pub use host::{
    ComponentHostError, default_engine, empty_linker, load_component, load_component_bytes,
    store_with_data,
};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(all(test, feature = "wasmtime"))]
mod tests;
