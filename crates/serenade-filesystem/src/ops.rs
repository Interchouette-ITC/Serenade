//! Create and remove paths.

use std::fs;
use std::path::Path;

use crate::error::FilesystemError;
use crate::path::{exists, is_dir};

/// Create a directory and parents (`mkdir -p`). No-op if it already exists as a directory.
///
/// # Errors
///
/// Returns [`FilesystemError`] when the path exists and is not a directory, or I/O fails.
pub fn mkdir(path: impl AsRef<Path>) -> Result<(), FilesystemError> {
    let path = path.as_ref();
    if is_dir(path) {
        return Ok(());
    }
    if exists(path) {
        return Err(FilesystemError::NotDirectory {
            path: path.to_path_buf(),
        });
    }
    fs::create_dir_all(path).map_err(|source| FilesystemError::io("mkdir", path, source))
}

/// Remove a file or an empty directory. No-op if missing.
///
/// # Errors
///
/// Returns [`FilesystemError`] on I/O failure (including non-empty directories).
pub fn remove(path: impl AsRef<Path>) -> Result<(), FilesystemError> {
    let path = path.as_ref();
    if !exists(path) {
        return Ok(());
    }
    if is_dir(path) {
        fs::remove_dir(path).map_err(|source| FilesystemError::io("remove", path, source))
    } else {
        fs::remove_file(path).map_err(|source| FilesystemError::io("remove", path, source))
    }
}

/// Recursively remove a directory tree, or a single file. No-op if missing.
///
/// # Errors
///
/// Returns [`FilesystemError`] on I/O failure.
pub fn remove_tree(path: impl AsRef<Path>) -> Result<(), FilesystemError> {
    let path = path.as_ref();
    if !exists(path) {
        return Ok(());
    }
    if is_dir(path) {
        fs::remove_dir_all(path).map_err(|source| FilesystemError::io("remove_tree", path, source))
    } else {
        fs::remove_file(path).map_err(|source| FilesystemError::io("remove_tree", path, source))
    }
}

/// Rename / move `from` to `to`.
///
/// # Errors
///
/// Returns [`FilesystemError`] when `from` is missing or rename fails.
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<(), FilesystemError> {
    let from = from.as_ref();
    let to = to.as_ref();
    if !exists(from) {
        return Err(FilesystemError::NotFound {
            path: from.to_path_buf(),
        });
    }
    if let Some(parent) = to.parent() {
        if !parent.as_os_str().is_empty() {
            mkdir(parent)?;
        }
    }
    fs::rename(from, to).map_err(|source| FilesystemError::io("rename", from, source))
}
