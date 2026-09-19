//! Notification channel kinds.

/// Delivery channel for a [`crate::Notification`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Channel {
    /// SMS (phone number recipient).
    Sms,
    /// Mobile push (device token recipient).
    Push,
}

impl Channel {
    /// Stable lowercase name (`sms`, `push`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sms => "sms",
            Self::Push => "push",
        }
    }
}
