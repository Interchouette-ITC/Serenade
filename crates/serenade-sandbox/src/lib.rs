//! Wasmer WASIX host helpers.
//!
//! Serenade owns engine plumbing. Product apps own package URLs, guest ABIs,
//! and fixtures. `FrameworkBundle` does not register this crate.
//!
//! Enable Cargo feature `wasix` (default).
//!
//! Cache root: env `SERENADE_WASMER_CACHE`, else a relative `.wasmer` next to
//! the calling crate's manifest when products wrap this helper.

#![forbid(unsafe_code)]

#[cfg(feature = "wasix")]
mod host;
#[cfg(feature = "wasix")]
mod output;
#[cfg(feature = "wasix")]
mod package_run;

#[cfg(feature = "wasix")]
pub use host::{
    load_webc_from, run_module, run_package, wasmer_cache_root, wasmer_cache_root_from,
};
#[cfg(feature = "wasix")]
pub use output::{GuestOutput, decode_json};
#[cfg(feature = "wasix")]
pub use package_run::PackageRun;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(all(test, feature = "wasix"))]
mod tests;
