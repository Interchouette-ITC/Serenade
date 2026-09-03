//! Kernel lifecycle errors.

use crate::KernelPhase;

/// Failure during kernel registration, compile, boot, or shutdown.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum KernelError {
    /// `Environment::from_name` received an unknown value.
    #[error("unknown environment `{0}`")]
    UnknownEnvironment(String),
    /// An operation was invoked in a phase that does not allow it.
    #[error("cannot {action} while kernel is {state}")]
    InvalidState {
        /// Attempted operation (`register`, `compile`, `boot`, or `shutdown`).
        action: &'static str,
        /// Current lifecycle phase.
        state: KernelPhase,
    },
    /// Two bundles were registered with the same [`Bundle::name`](crate::Bundle::name).
    #[error("bundle `{0}` is already registered")]
    DuplicateBundle(&'static str),
    /// A bundle returned an error from `build`, `boot`, or `shutdown`.
    #[error("bundle `{bundle}` failed during {phase}: {message}")]
    Bundle {
        /// Bundle name.
        bundle: &'static str,
        /// Lifecycle method that failed.
        phase: &'static str,
        /// Underlying error text.
        message: String,
    },
}
