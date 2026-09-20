//! Finder errors.

use std::path::PathBuf;

/// Failure while configuring or walking a finder.
#[derive(Debug, thiserror::Error)]
pub enum FinderError {
    /// No search roots were configured.
    #[error("finder has no search paths; call `in_path` first")]
    EmptyRoots,
    /// A configured root is missing or not a directory.
    #[error("finder root is not a directory: {path}")]
    InvalidRoot {
        /// Path that failed validation.
        path: PathBuf,
    },
    /// Walk failed (permissions, I/O).
    #[error("finder walk on `{path}`: {source}")]
    Walk {
        /// Path being walked when the error occurred.
        path: PathBuf,
        /// Source I/O error.
        #[source]
        source: std::io::Error,
    },
}

impl PartialEq for FinderError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::EmptyRoots, Self::EmptyRoots) => true,
            (Self::InvalidRoot { path: a }, Self::InvalidRoot { path: b }) => a == b,
            (
                Self::Walk {
                    path: a,
                    source: a_src,
                },
                Self::Walk {
                    path: b,
                    source: b_src,
                },
            ) => a == b && a_src.kind() == b_src.kind(),
            _ => false,
        }
    }
}

impl Eq for FinderError {}
