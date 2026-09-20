//! Copy files and mirror directory trees.

use std::fs;
use std::path::Path;

use crate::error::FilesystemError;
use crate::ops::mkdir;
use crate::path::{exists, is_dir, is_file};

/// Copy a file to `to` (overwrites). Creates parent directories of `to`.
///
/// # Errors
///
/// Returns [`FilesystemError`] when `from` is missing / not a file, or I/O fails.
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<(), FilesystemError> {
    let from = from.as_ref();
    let to = to.as_ref();
    if !exists(from) {
        return Err(FilesystemError::NotFound {
            path: from.to_path_buf(),
        });
    }
    if !is_file(from) {
        return Err(FilesystemError::NotFile {
            path: from.to_path_buf(),
        });
    }
    if let Some(parent) = to.parent() {
        if !parent.as_os_str().is_empty() {
            mkdir(parent)?;
        }
    }
    fs::copy(from, to).map_err(|source| FilesystemError::io("copy", from, source))?;
    Ok(())
}

/// Recursively copy directory `from` into `to` (creates `to`).
///
/// Existing files under `to` are overwritten. Does not delete extras in `to`.
///
/// # Errors
///
/// Returns [`FilesystemError`] when `from` is missing / not a directory, or I/O fails.
pub fn mirror(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<(), FilesystemError> {
    let from = from.as_ref();
    let to = to.as_ref();
    if !exists(from) {
        return Err(FilesystemError::NotFound {
            path: from.to_path_buf(),
        });
    }
    if !is_dir(from) {
        return Err(FilesystemError::NotDirectory {
            path: from.to_path_buf(),
        });
    }
    mkdir(to)?;
    mirror_into(from, to)
}

fn mirror_into(from: &Path, to: &Path) -> Result<(), FilesystemError> {
    let entries =
        fs::read_dir(from).map_err(|source| FilesystemError::io("mirror", from, source))?;
    for entry in entries {
        let entry = entry.map_err(|source| FilesystemError::io("mirror", from, source))?;
        let src = entry.path();
        let name = entry.file_name();
        let dst = to.join(&name);
        let file_type = entry
            .file_type()
            .map_err(|source| FilesystemError::io("mirror", &src, source))?;
        if file_type.is_dir() {
            mkdir(&dst)?;
            mirror_into(&src, &dst)?;
        } else if file_type.is_file() {
            copy(&src, &dst)?;
        }
        // Symlinks / other types: skip (stable, predictable).
    }
    Ok(())
}
