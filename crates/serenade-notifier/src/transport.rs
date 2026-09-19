//! Sync notifier transport contract.

use crate::{Channel, Notification, NotifierError};

/// Sends a [`Notification`] synchronously.
pub trait Transport: Send + Sync {
    /// Whether this transport handles `channel`.
    fn supports(&self, channel: Channel) -> bool;

    /// Delivers `notification` through this backend.
    ///
    /// # Errors
    ///
    /// Returns [`NotifierError`] when validation or delivery fails.
    fn send(&self, notification: &Notification) -> Result<(), NotifierError>;
}
