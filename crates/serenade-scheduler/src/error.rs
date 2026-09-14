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
    /// No [`crate::ScheduleHandler`] is registered for a due schedule id.
    #[error("no schedule handler registered for `{id}`")]
    MissingHandler {
        /// Schedule id that fired without a handler.
        id: String,
    },
    /// A registered handler failed while processing a due job.
    #[error("schedule handler `{id}` failed: {message}")]
    Handler {
        /// Schedule id being handled.
        id: String,
        /// Reason text.
        message: String,
    },
}
