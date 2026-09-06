//! Sync in-process command and event message bus.
//!
//! Commands have exactly one handler. Events fan out to zero or more handlers.
//! Optional middleware wraps each dispatch (logging and validation hooks included).
//! Async queue transports are a later slice of the messenger epic.

mod bus;
mod error;
mod handler;
mod message;
mod middleware;

pub use bus::MessageBus;
pub use error::MessengerError;
pub use handler::{CommandHandler, EventHandler};
pub use message::{Command, Event, Message};
pub use middleware::{
    DispatchContext, DispatchKind, LogSink, LoggingMiddleware, Middleware, ValidateHook,
    ValidationMiddleware,
};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
