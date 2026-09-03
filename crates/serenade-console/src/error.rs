//! Console errors.

use thiserror::Error;

/// Failures while parsing argv or running a command.
#[derive(Debug, Error)]
pub enum ConsoleError {
    /// Unknown command name.
    #[error("command not found: {0}")]
    NotFound(String),
    /// Invalid `--env` value.
    #[error("invalid environment: {0}")]
    InvalidEnvironment(String),
    /// Command execution failed.
    #[error("{0}")]
    Failed(String),
    /// Terminal / TUI I/O failed.
    #[error("console I/O: {0}")]
    Io(#[from] std::io::Error),
}
