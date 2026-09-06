//! Observability errors.

use thiserror::Error;

/// Failures while configuring or installing the global subscriber.
#[derive(Debug, Error)]
pub enum ObservabilityError {
    /// Both stderr and file sinks were disabled.
    #[error("logging requires at least one sink (stderr or file)")]
    NoSinks,

    /// Environment name used for the log file was empty.
    #[error("logging environment name must not be empty")]
    EmptyEnvironment,

    /// Creating the log directory failed.
    #[error("could not create log directory `{path}`: {source}")]
    CreateDir {
        /// Path that could not be created.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Filter directives could not be parsed.
    #[error("invalid log filter directives: {0}")]
    InvalidFilter(String),

    /// A global tracing subscriber is already installed.
    #[error("tracing subscriber already initialized")]
    AlreadyInitialized,
}
