//! Transport that only accepts SMS.

use crate::null::validate_for_send;
use crate::{Channel, Notification, NotifierError, Transport};

/// Rejects push; accepts SMS (useful when only an SMS vendor is wired).
#[derive(Clone, Copy, Debug, Default)]
pub struct SmsOnlyTransport;

impl SmsOnlyTransport {
    /// Creates an SMS-only transport.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Transport for SmsOnlyTransport {
    fn supports(&self, channel: Channel) -> bool {
        matches!(channel, Channel::Sms)
    }

    fn send(&self, notification: &Notification) -> Result<(), NotifierError> {
        if !self.supports(notification.channel()) {
            return Err(NotifierError::UnsupportedChannel {
                channel: notification.channel().as_str().to_owned(),
            });
        }
        validate_for_send(notification)
    }
}
