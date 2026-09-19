//! In-memory transport that records sent notifications (tests / local).

use std::sync::Mutex;

use crate::null::validate_for_send;
use crate::{Channel, Notification, NotifierError, Transport};

/// Records every successful send for inspection in tests and local apps.
#[derive(Debug, Default)]
pub struct MemoryTransport {
    sent: Mutex<Vec<Notification>>,
}

impl MemoryTransport {
    /// Empty recorder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of notifications accepted so far.
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned.
    #[must_use]
    pub fn sent(&self) -> Vec<Notification> {
        self.sent.lock().expect("memory transport lock").clone()
    }

    /// Clears recorded notifications.
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned.
    pub fn clear(&self) {
        self.sent.lock().expect("memory transport lock").clear();
    }
}

impl Transport for MemoryTransport {
    fn supports(&self, channel: Channel) -> bool {
        matches!(channel, Channel::Sms | Channel::Push)
    }

    fn send(&self, notification: &Notification) -> Result<(), NotifierError> {
        validate_for_send(notification)?;
        self.sent
            .lock()
            .expect("memory transport lock")
            .push(notification.clone());
        Ok(())
    }
}
