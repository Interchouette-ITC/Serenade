//! Dependency injection errors.

/// Failure while building or resolving the service container.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum DiError {
    /// A service id was requested but never registered.
    #[error("service `{0}` is not registered")]
    NotFound(String),
    /// A parameter key is missing from the bag.
    #[error("parameter `{0}` is not set")]
    ParameterNotFound(String),
    /// Two definitions share the same id.
    #[error("service `{0}` is already registered")]
    DuplicateService(String),
    /// An alias points at a missing or looping target.
    #[error("alias `{alias}` has invalid target `{target}`")]
    InvalidAlias {
        /// Alias id.
        alias: String,
        /// Target service id.
        target: String,
    },
    /// Static or runtime cycle among service dependencies.
    #[error("circular service dependency: {0}")]
    CircularDependency(String),
    /// A factory returned an unexpected failure.
    #[error("service `{service}` factory failed: {message}")]
    Factory {
        /// Service id being built.
        service: String,
        /// Underlying error text.
        message: String,
    },
}
