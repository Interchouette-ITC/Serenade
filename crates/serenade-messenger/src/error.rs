//! Messenger bus errors.

/// Failure while registering or dispatching a message.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum MessengerError {
    /// No command handler is registered for this message name.
    #[error("no command handler for `{name}`")]
    UnknownCommand {
        /// Message name from [`crate::Message::NAME`].
        name: &'static str,
    },
    /// A second command handler was registered for the same name.
    #[error("duplicate command handler for `{name}`")]
    DuplicateCommand {
        /// Message name from [`crate::Message::NAME`].
        name: &'static str,
    },
    /// A handler returned a failure.
    #[error("handler for `{name}` failed: {message}")]
    Handler {
        /// Message name being handled.
        name: &'static str,
        /// Underlying error text.
        message: String,
    },
    /// Middleware rejected the message before the handler ran.
    #[error("message `{name}` rejected: {message}")]
    Rejected {
        /// Message name being rejected.
        name: &'static str,
        /// Rejection reason.
        message: String,
    },
}
