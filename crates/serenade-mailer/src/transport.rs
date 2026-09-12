//! Sync mail transport contract.

use crate::{Email, MailerError};

/// Sends an [`Email`] synchronously.
pub trait Transport: Send + Sync {
    /// Delivers `email` through this backend.
    ///
    /// # Errors
    ///
    /// Returns [`MailerError`] when validation or delivery fails.
    fn send(&self, email: &Email) -> Result<(), MailerError>;
}
