//! Directory walking with name glob and type filters (Symfony Finder shaped).
//!
//! - [`Finder`] - `in_path`, `files` / `directories`, `name`, depth, `ignore_dotfiles`
//! - [`Finder::collect`] - sorted matching paths
//!
//! # Examples
//!
//! ```
//! use serenade_finder::Finder;
//! use std::fs;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let dir = tempfile_dir()?;
//! fs::write(dir.join("a.rs"), b"")?;
//! fs::write(dir.join("b.txt"), b"")?;
//! let found = Finder::new().in_path(&dir).files().name("*.rs").collect()?;
//! assert_eq!(found.len(), 1);
//! # Ok(())
//! # }
//! # fn tempfile_dir() -> std::io::Result<std::path::PathBuf> {
//! #   let dir = std::env::temp_dir().join(format!("serenade-finder-doc-{}", std::process::id()));
//! #   fs::create_dir_all(&dir)?;
//! #   Ok(dir)
//! # }
//! ```

mod error;
mod finder;
mod glob;

pub use error::FinderError;
pub use finder::{EntryKind, Finder};
pub use glob::name_matches;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
