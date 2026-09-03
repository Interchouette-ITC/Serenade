//! Console errors.

use thiserror::Error;

/// Failures while parsing argv or running a command.
#[derive(Debug, Error)]
pub enum ConsoleError {
    /// Unknown command name.
    #[error("command `{0}` was not found; run with no arguments to list commands")]
    NotFound(String),
    /// Invalid `--env` value.
    #[error("invalid `--env` value: {0}")]
    InvalidEnvironment(String),
    /// Command execution failed.
    #[error("command failed: {0}")]
    Failed(String),
    /// Terminal / TUI I/O failed.
    #[error("console I/O failed: {0}")]
    Io(#[from] std::io::Error),
}
