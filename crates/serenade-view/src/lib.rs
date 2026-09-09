//! HTML view helpers: reverse routing, assets, and partials.
//!
//! Escape stays in [`serenade_form`]; this crate re-exports it for one import surface.
//! No mandatory template engine - apps compose HTML with these helpers.

mod asset;
mod error;
mod partial;
mod path;

pub use asset::{AssetConfig, asset};
pub use error::ViewError;
pub use partial::partial;
pub use path::path;
pub use serenade_form::{escape_attr, escape_html};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
