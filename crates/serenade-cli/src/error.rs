//! CLI errors.

use std::path::PathBuf;

/// Failures from `serenade new` / `serenade recipe`.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// Destination already exists / is not empty.
    #[error("destination `{0}` already exists; use --force to overwrite")]
    DestinationExists(PathBuf),
    /// Target file would be overwritten without `--force`.
    #[error("refusing to overwrite `{0}`; use --force")]
    FileExists(PathBuf),
    /// Unknown recipe id.
    #[error("unknown recipe `{0}`")]
    UnknownRecipe(String),
    /// Invalid recipe metadata.
    #[error("invalid recipe: {0}")]
    InvalidRecipe(String),
    /// Embedded asset missing.
    #[error("missing embedded file `{0}`")]
    MissingAsset(String),
    /// Filesystem IO.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// `cargo add` failed.
    #[error("cargo add failed: {0}")]
    Cargo(String),
    /// Invalid package / directory name.
    #[error("invalid name `{0}`")]
    InvalidName(String),
}
