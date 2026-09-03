//! Event and subscriber contracts.

use crate::EventError;

/// Synchronous application or infrastructure event.
pub trait Event: Send + Sync {
    /// Stable event name used for subscriber matching (for example `cart.updated`).
    fn name(&self) -> &'static str;
}

/// Handles events of one name. Higher [`Self::priority`] runs first.
pub trait EventSubscriber: Send + Sync {
    /// Event name this subscriber listens to.
    fn event_name(&self) -> &'static str;

    /// Dispatch order. Higher values run first. Default is `0`.
    fn priority(&self) -> i32 {
        0
    }

    /// Handles a matching event.
    ///
    /// # Errors
    ///
    /// Return [`EventError`] when handling fails. Remaining subscribers still run.
    fn handle(&self, event: &dyn Event) -> Result<(), EventError>;
}
