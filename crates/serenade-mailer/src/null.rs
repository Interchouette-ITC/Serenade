//! Null transport (discards messages).

use crate::{Email, MailerError, Transport};

/// Discards every message (Symfony `NullTransport`).
#[derive(Clone, Copy, Debug, Default)]
pub struct NullTransport;

impl NullTransport {
    /// Creates a null transport.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Transport for NullTransport {
    fn send(&self, email: &Email) -> Result<(), MailerError> {
        validate_for_send(email)
    }
}

pub fn validate_for_send(email: &Email) -> Result<(), MailerError> {
    if email.from_addresses().is_empty() {
        return Err(MailerError::MissingSender);
    }
    if email.to_addresses().is_empty()
        && email.cc_addresses().is_empty()
        && email.bcc_addresses().is_empty()
    {
        return Err(MailerError::MissingRecipient);
    }
    Ok(())
}
