//! [`SchedulerError`] variants.

/// Failure while building or running a schedule.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    /// Schedule id is empty or otherwise invalid.
    #[error("invalid schedule id `{id}`: {message}")]
    InvalidId {
        /// Id that failed validation.
        id: String,
        /// Reason text.
        message: String,
    },
    /// Trigger configuration is invalid.
    #[error("invalid schedule trigger: {message}")]
    InvalidTrigger {
        /// Reason text.
        message: String,
    },
}
