//! Filesystem errors.

use std::path::PathBuf;

/// Failure while reading, writing, or mirroring paths.
#[derive(Debug, thiserror::Error)]
pub enum FilesystemError {
    /// Underlying I/O failure.
    #[error("filesystem `{op}` on `{path}`: {source}")]
    Io {
        /// Operation name (`mkdir`, `remove`, `copy`, …).
        op: &'static str,
        /// Path involved.
        path: PathBuf,
        /// Source I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Path does not exist when required.
    #[error("path does not exist: {path}")]
    NotFound {
        /// Missing path.
        path: PathBuf,
    },
    /// Expected a directory.
    #[error("not a directory: {path}")]
    NotDirectory {
        /// Path that is not a directory.
        path: PathBuf,
    },
    /// Expected a file.
    #[error("not a file: {path}")]
    NotFile {
        /// Path that is not a file.
        path: PathBuf,
    },
}

impl FilesystemError {
    /// Wrap an I/O error with operation and path context.
    #[must_use]
    pub fn io(op: &'static str, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            op,
            path: path.into(),
            source,
        }
    }
}

impl PartialEq for FilesystemError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Io {
                    op: a_op,
                    path: a_path,
                    source: a_src,
                },
                Self::Io {
                    op: b_op,
                    path: b_path,
                    source: b_src,
                },
            ) => a_op == b_op && a_path == b_path && a_src.kind() == b_src.kind(),
            (Self::NotFound { path: a }, Self::NotFound { path: b })
            | (Self::NotDirectory { path: a }, Self::NotDirectory { path: b })
            | (Self::NotFile { path: a }, Self::NotFile { path: b }) => a == b,
            _ => false,
        }
    }
}

impl Eq for FilesystemError {}
