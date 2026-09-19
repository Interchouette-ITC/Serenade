//! Notification payloads for SMS and push.

use crate::Channel;

/// SMS payload (phone + body).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmsMessage {
    to: String,
    body: String,
}

impl SmsMessage {
    /// Builds an SMS message.
    #[must_use]
    pub fn new(to: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            body: body.into(),
        }
    }

    /// Phone / MSISDN recipient.
    #[must_use]
    pub fn to(&self) -> &str {
        &self.to
    }

    /// Message body.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// Push payload (device token + title + body).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PushMessage {
    device_token: String,
    title: String,
    body: String,
}

impl PushMessage {
    /// Builds a push message.
    #[must_use]
    pub fn new(
        device_token: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            device_token: device_token.into(),
            title: title.into(),
            body: body.into(),
        }
    }

    /// Device token / registration id.
    #[must_use]
    pub fn device_token(&self) -> &str {
        &self.device_token
    }

    /// Notification title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Notification body.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// Outgoing notification before a transport sends it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Notification {
    /// SMS channel.
    Sms(SmsMessage),
    /// Push channel.
    Push(PushMessage),
}

impl Notification {
    /// Channel for this notification.
    #[must_use]
    pub const fn channel(&self) -> Channel {
        match self {
            Self::Sms(_) => Channel::Sms,
            Self::Push(_) => Channel::Push,
        }
    }

    /// SMS constructor.
    #[must_use]
    pub fn sms(to: impl Into<String>, body: impl Into<String>) -> Self {
        Self::Sms(SmsMessage::new(to, body))
    }

    /// Push constructor.
    #[must_use]
    pub fn push(
        device_token: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self::Push(PushMessage::new(device_token, title, body))
    }
}
