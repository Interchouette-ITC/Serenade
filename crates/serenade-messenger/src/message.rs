//! Message marker traits for the sync bus.

/// Envelope carried by the messenger bus.
///
/// `NAME` is the stable routing key used for handler registration and dispatch.
pub trait Message: Send + Sync + 'static {
    /// Stable message name (for example `cart.place_order`).
    const NAME: &'static str;

    /// Returns [`Self::NAME`].
    fn name(&self) -> &'static str {
        Self::NAME
    }
}

/// Mutation with exactly one handler.
pub trait Command: Message {}

/// Notification with zero or more handlers (fan-out).
pub trait Event: Message {}
