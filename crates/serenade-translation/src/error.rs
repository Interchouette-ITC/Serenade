//! Translation errors.

use std::path::PathBuf;

/// Failure while loading catalogues or negotiating a locale.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum TranslationError {
    /// Locale tag is empty or not a valid BCP 47 language tag shape.
    #[error("invalid locale `{tag}`")]
    InvalidLocale {
        /// Offending tag.
        tag: String,
    },
    /// Catalogue file could not be read or parsed.
    #[error("failed to load catalogue from {path}: {detail}")]
    Load {
        /// Source path.
        path: PathBuf,
        /// Parser or I/O detail.
        detail: String,
    },
    /// Loader does not support the requested format.
    #[error("unsupported catalogue format `{format}`")]
    UnsupportedFormat {
        /// Requested format id (`toml`, `json`, …).
        format: String,
    },
    /// Directory walk failed.
    #[error("catalogue directory error for {path}: {detail}")]
    Directory {
        /// Directory path.
        path: PathBuf,
        /// OS or walk detail.
        detail: String,
    },
}
