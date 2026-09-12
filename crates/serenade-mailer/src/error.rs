//! Mailer errors.

/// Failure while building or sending an email message.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum MailerError {
    /// Address is empty or not a plausible mailbox.
    #[error("invalid address: {message}")]
    InvalidAddress {
        /// Reason text.
        message: String,
    },
    /// Message has no From address.
    #[error("email has no From address")]
    MissingSender,
    /// Message has no To / Cc / Bcc recipient.
    #[error("email has no recipient")]
    MissingRecipient,
    /// Transport backend failed.
    #[error("mailer transport: {message}")]
    Transport {
        /// Reason text.
        message: String,
    },
    /// Local I/O failed (for example file transport).
    #[error("mailer io: {message}")]
    Io {
        /// Reason text.
        message: String,
    },
}
