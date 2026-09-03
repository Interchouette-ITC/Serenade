//! Event dispatcher errors.

/// Failure while dispatching or handling an event.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum EventError {
    /// A subscriber returned a failure.
    #[error("subscriber `{subscriber}` failed on `{event}`: {message}")]
    Subscriber {
        /// Subscriber diagnostic name (event name plus priority).
        subscriber: String,
        /// Event name.
        event: &'static str,
        /// Underlying error text.
        message: String,
    },
}
