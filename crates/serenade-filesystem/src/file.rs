//! Read and write file contents.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::error::FilesystemError;
use crate::ops::mkdir;
use crate::path::{exists, is_file};

/// Write `contents` atomically-ish: ensure parent dir, then write (overwrite).
///
/// # Errors
///
/// Returns [`FilesystemError`] on I/O failure.
pub fn dump_file(
    path: impl AsRef<Path>,
    contents: impl AsRef<[u8]>,
) -> Result<(), FilesystemError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            mkdir(parent)?;
        }
    }
    fs::write(path, contents.as_ref())
        .map_err(|source| FilesystemError::io("dump_file", path, source))
}

/// Append `contents` to a file, creating it (and parents) if needed.
///
/// # Errors
///
/// Returns [`FilesystemError`] on I/O failure.
pub fn append_to_file(
    path: impl AsRef<Path>,
    contents: impl AsRef<[u8]>,
) -> Result<(), FilesystemError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            mkdir(parent)?;
        }
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|source| FilesystemError::io("append_to_file", path, source))?;
    file.write_all(contents.as_ref())
        .map_err(|source| FilesystemError::io("append_to_file", path, source))?;
    Ok(())
}

/// Read entire file as bytes.
///
/// # Errors
///
/// Returns [`FilesystemError`] when missing, not a file, or I/O fails.
pub fn read(path: impl AsRef<Path>) -> Result<Vec<u8>, FilesystemError> {
    let path = path.as_ref();
    if !exists(path) {
        return Err(FilesystemError::NotFound {
            path: path.to_path_buf(),
        });
    }
    if !is_file(path) {
        return Err(FilesystemError::NotFile {
            path: path.to_path_buf(),
        });
    }
    fs::read(path).map_err(|source| FilesystemError::io("read", path, source))
}

/// Read entire file as UTF-8 string.
///
/// # Errors
///
/// Returns [`FilesystemError`] when missing, not a file, or I/O fails.
pub fn read_to_string(path: impl AsRef<Path>) -> Result<String, FilesystemError> {
    let path = path.as_ref();
    if !exists(path) {
        return Err(FilesystemError::NotFound {
            path: path.to_path_buf(),
        });
    }
    if !is_file(path) {
        return Err(FilesystemError::NotFile {
            path: path.to_path_buf(),
        });
    }
    fs::read_to_string(path).map_err(|source| FilesystemError::io("read_to_string", path, source))
}

/// Create an empty file (and parents) if missing; update mtime when present.
///
/// # Errors
///
/// Returns [`FilesystemError`] on I/O failure.
pub fn touch(path: impl AsRef<Path>) -> Result<(), FilesystemError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            mkdir(parent)?;
        }
    }
    if exists(path) {
        OpenOptions::new()
            .write(true)
            .open(path)
            .map_err(|source| FilesystemError::io("touch", path, source))?;
        return Ok(());
    }
    File::create(path).map_err(|source| FilesystemError::io("touch", path, source))?;
    Ok(())
}
