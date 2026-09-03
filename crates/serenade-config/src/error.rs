//! Configuration load and merge errors.

use std::path::PathBuf;

/// Failure while loading, merging, or interpolating configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Filesystem error while reading a config file or packages directory.
    #[error("config io error at {path}: {source}")]
    Io {
        /// Path that failed.
        path: PathBuf,
        /// Underlying io error.
        #[source]
        source: std::io::Error,
    },
    /// YAML document could not be parsed.
    #[error("invalid YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    /// TOML document could not be parsed.
    #[error("invalid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    /// JSON conversion failed while normalizing a document.
    #[error("invalid config document: {0}")]
    Json(#[from] serde_json::Error),
    /// Root value is not a mapping.
    #[error("config root must be a mapping")]
    InvalidRoot,
    /// File extension is not YAML or TOML.
    #[error("unsupported config format: {path}")]
    UnsupportedFormat {
        /// Path with an unknown extension.
        path: PathBuf,
    },
    /// `${VAR}` placeholder is malformed.
    #[error("invalid environment interpolation")]
    InvalidInterpolation,
    /// Environment variable is required and unset.
    #[error("missing environment variable `{name}`")]
    MissingEnvironment {
        /// Variable name.
        name: String,
    },
    /// A `.env` file could not be loaded.
    #[error("dotenv error at {path}: {message}")]
    Dotenv {
        /// Path that failed.
        path: PathBuf,
        /// Human-readable parse or IO message.
        message: String,
    },
}
