//! Path helpers, dump/mirror, and temp utilities (Symfony Filesystem shaped).
//!
//! - [`exists`], [`is_file`], [`is_dir`]
//! - [`mkdir`], [`remove`], [`remove_tree`], [`rename`]
//! - [`dump_file`], [`append_to_file`], [`read`], [`read_to_string`], [`touch`]
//! - [`copy`], [`mirror`]
//! - [`temp_dir`], [`temp_file`], …
//!
//! # Examples
//!
//! ```
//! use serenade_filesystem::{dump_file, mkdir, read_to_string, temp_dir};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let dir = temp_dir()?;
//! let nested = dir.path().join("a/b");
//! mkdir(&nested)?;
//! let file = nested.join("hello.txt");
//! dump_file(&file, b"hi")?;
//! assert_eq!(read_to_string(&file)?, "hi");
//! # Ok(())
//! # }
//! ```

mod error;
mod file;
mod mirror;
mod ops;
mod path;
mod temp;

pub use error::FilesystemError;
pub use file::{append_to_file, dump_file, read, read_to_string, touch};
pub use mirror::{copy, mirror};
pub use ops::{mkdir, remove, remove_tree, rename};
pub use path::{exists, is_dir, is_file};
pub use temp::{temp_dir, temp_dir_in, temp_file, temp_file_in};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
