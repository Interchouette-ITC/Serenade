//! Sync in-process command and event message bus.
//!
//! Commands have exactly one handler. Events fan out to zero or more handlers.
//! Async queue transports and middleware pipelines are out of scope for this crate slice.

mod bus;
mod error;
mod handler;
mod message;

pub use bus::MessageBus;
pub use error::MessengerError;
pub use handler::{CommandHandler, EventHandler};
pub use message::{Command, Event, Message};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
