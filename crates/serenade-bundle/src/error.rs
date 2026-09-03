//! Bundle composition errors.

use serenade_config::ConfigError;
use serenade_di::DiError;
use serenade_kernel::KernelError;

/// Failure while loading extensions or compiling the framework container.
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    /// Kernel lifecycle failure.
    #[error(transparent)]
    Kernel(#[from] KernelError),
    /// Configuration load or interpolation failure.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// Container build or resolve failure.
    #[error(transparent)]
    Di(#[from] DiError),
    /// Extension reported a failure.
    #[error("extension `{alias}` failed: {message}")]
    Extension {
        /// Extension alias (package key).
        alias: &'static str,
        /// Underlying error text.
        message: String,
    },
}
