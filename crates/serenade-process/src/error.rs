//! Process errors.

use std::path::PathBuf;
use std::time::Duration;

/// Failure while building or running a child process.
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    /// Command name is empty.
    #[error("process command must not be empty")]
    EmptyCommand,
    /// Failed to spawn or wait on the child.
    #[error("process `{command}`: {source}")]
    Io {
        /// Executable name.
        command: String,
        /// Source I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Child exceeded the configured timeout and was killed.
    #[error("process `{command}` timed out after {timeout:?}")]
    TimedOut {
        /// Executable name.
        command: String,
        /// Timeout that was exceeded.
        timeout: Duration,
    },
    /// `must_run` failed because the exit status was non-zero.
    #[error("process `{command}` failed with exit code {code:?}")]
    Failed {
        /// Executable name.
        command: String,
        /// Exit code when available.
        code: Option<i32>,
    },
    /// Working directory does not exist or is not a directory.
    #[error("process cwd is invalid: {path}")]
    InvalidCwd {
        /// Requested working directory.
        path: PathBuf,
    },
}

impl PartialEq for ProcessError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::EmptyCommand, Self::EmptyCommand) => true,
            (
                Self::Io {
                    command: a,
                    source: a_src,
                },
                Self::Io {
                    command: b,
                    source: b_src,
                },
            ) => a == b && a_src.kind() == b_src.kind(),
            (
                Self::TimedOut {
                    command: a,
                    timeout: a_t,
                },
                Self::TimedOut {
                    command: b,
                    timeout: b_t,
                },
            ) => a == b && a_t == b_t,
            (
                Self::Failed {
                    command: a,
                    code: a_c,
                },
                Self::Failed {
                    command: b,
                    code: b_c,
                },
            ) => a == b && a_c == b_c,
            (Self::InvalidCwd { path: a }, Self::InvalidCwd { path: b }) => a == b,
            _ => false,
        }
    }
}

impl Eq for ProcessError {}
