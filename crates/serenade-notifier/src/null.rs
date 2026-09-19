//! Null transport (validates then discards).

use crate::{Channel, Notification, NotifierError, Transport};

/// Discards every notification after validation (Symfony `NullTransport` analogue).
#[derive(Clone, Copy, Debug, Default)]
pub struct NullTransport;

impl NullTransport {
    /// Creates a null transport that supports SMS and push.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Transport for NullTransport {
    fn supports(&self, channel: Channel) -> bool {
        matches!(channel, Channel::Sms | Channel::Push)
    }

    fn send(&self, notification: &Notification) -> Result<(), NotifierError> {
        validate_for_send(notification)
    }
}

/// Shared validation for SMS / push payloads.
pub fn validate_for_send(notification: &Notification) -> Result<(), NotifierError> {
    match notification {
        Notification::Sms(message) => {
            if message.to().trim().is_empty() {
                return Err(NotifierError::MissingRecipient);
            }
            if message.body().trim().is_empty() {
                return Err(NotifierError::MissingBody);
            }
        }
        Notification::Push(message) => {
            if message.device_token().trim().is_empty() {
                return Err(NotifierError::MissingRecipient);
            }
            if message.body().trim().is_empty() {
                return Err(NotifierError::MissingBody);
            }
        }
    }
    Ok(())
}
