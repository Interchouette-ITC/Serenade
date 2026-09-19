//! Notifier errors.

/// Failure while building or sending a notification.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum NotifierError {
    /// Recipient is empty.
    #[error("notification has no recipient")]
    MissingRecipient,
    /// Message body is empty.
    #[error("notification has no body")]
    MissingBody,
    /// Transport does not support this channel.
    #[error("channel {channel} is not supported by this transport")]
    UnsupportedChannel {
        /// Channel name.
        channel: String,
    },
    /// Transport backend failed.
    #[error("notifier transport: {message}")]
    Transport {
        /// Reason text.
        message: String,
    },
}
