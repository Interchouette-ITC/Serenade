//! Validator errors.

/// Failure while running constraints.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ValidatorError {
    /// Validation produced one or more violations.
    #[error("validation failed with {count} violation(s)")]
    Violations {
        /// Number of collected violations.
        count: usize,
    },
}
