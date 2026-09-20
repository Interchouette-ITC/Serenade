//! Temporary files and directories.

use std::path::{Path, PathBuf};

use tempfile::{Builder, NamedTempFile, TempDir};

use crate::error::FilesystemError;

/// Create a temporary directory (deleted when [`TempDir`] is dropped).
///
/// # Errors
///
/// Returns [`FilesystemError`] when the OS temp directory cannot be used.
pub fn temp_dir() -> Result<TempDir, FilesystemError> {
    TempDir::new()
        .map_err(|source| FilesystemError::io("temp_dir", PathBuf::from("<temp>"), source))
}

/// Create a temporary directory under `parent`.
///
/// # Errors
///
/// Returns [`FilesystemError`] on I/O failure.
pub fn temp_dir_in(parent: impl AsRef<Path>) -> Result<TempDir, FilesystemError> {
    let parent = parent.as_ref();
    TempDir::new_in(parent).map_err(|source| FilesystemError::io("temp_dir_in", parent, source))
}

/// Create a named temporary file (deleted when [`NamedTempFile`] is dropped).
///
/// # Errors
///
/// Returns [`FilesystemError`] when creation fails.
pub fn temp_file() -> Result<NamedTempFile, FilesystemError> {
    NamedTempFile::new()
        .map_err(|source| FilesystemError::io("temp_file", PathBuf::from("<temp>"), source))
}

/// Create a named temporary file under `parent` with optional `prefix` / `suffix`.
///
/// # Errors
///
/// Returns [`FilesystemError`] on I/O failure.
pub fn temp_file_in(
    parent: impl AsRef<Path>,
    prefix: &str,
    suffix: &str,
) -> Result<NamedTempFile, FilesystemError> {
    let parent = parent.as_ref();
    Builder::new()
        .prefix(prefix)
        .suffix(suffix)
        .tempfile_in(parent)
        .map_err(|source| FilesystemError::io("temp_file_in", parent, source))
}
