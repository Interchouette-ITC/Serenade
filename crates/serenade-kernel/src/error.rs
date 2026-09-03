//! Kernel lifecycle errors.

use crate::KernelPhase;

/// Failure during kernel registration, compile, boot, or shutdown.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum KernelError {
    /// `Environment::from_name` received an empty name.
    #[error("unknown environment `{0}` (use `dev`, `test`, `prod`, or a non-empty custom name)")]
    UnknownEnvironment(String),
    /// An operation was invoked in a phase that does not allow it.
    #[error("cannot `{action}` while kernel is in phase `{state}`")]
    InvalidState {
        /// Attempted operation (`register`, `compile`, `boot`, or `shutdown`).
        action: &'static str,
        /// Current lifecycle phase.
        state: KernelPhase,
    },
    /// Two bundles were registered with the same [`BundleInterface::name`](crate::BundleInterface::name).
    #[error("bundle `{0}` is already registered")]
    DuplicateBundle(&'static str),
    /// A bundle listed a dependency that was never registered.
    #[error("bundle `{bundle}` depends on unknown bundle `{dependency}`")]
    UnknownBundleDependency {
        /// Bundle that declared the dependency.
        bundle: &'static str,
        /// Missing dependency name.
        dependency: &'static str,
    },
    /// Bundle dependencies form a cycle; `0` is one member of the cycle.
    #[error("cyclic bundle dependency involving `{0}`")]
    CyclicBundleDependency(&'static str),
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
