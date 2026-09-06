//! Typed command and event handlers.

use crate::{Command, Event, MessengerError};

/// Handles one command type. Register at most one per [`crate::Message::NAME`].
pub trait CommandHandler<C: Command>: Send + Sync {
    /// Handles `command`.
    ///
    /// # Errors
    ///
    /// Return [`MessengerError`] when handling fails.
    fn handle(&self, command: &C) -> Result<(), MessengerError>;
}

/// Handles one event type. Many handlers may share the same [`crate::Message::NAME`].
pub trait EventHandler<E: Event>: Send + Sync {
    /// Handles `event`.
    ///
    /// # Errors
    ///
    /// Return [`MessengerError`] when handling fails.
    fn handle(&self, event: &E) -> Result<(), MessengerError>;
}
