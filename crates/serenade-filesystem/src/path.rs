//! Path existence and type checks.

use std::path::Path;

/// Whether `path` exists (follows symlinks for the final component).
#[must_use]
pub fn exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

/// Whether `path` is an existing regular file.
#[must_use]
pub fn is_file(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_file()
}

/// Whether `path` is an existing directory.
#[must_use]
pub fn is_dir(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_dir()
}
