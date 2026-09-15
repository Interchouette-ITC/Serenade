//! Sync in-process command and event message bus.
//!
//! Commands have exactly one handler. Events fan out to zero or more handlers.
//! Optional middleware wraps each dispatch (logging and validation hooks included).
//! Async queue backends implement [`Transport`]; [`InMemoryTransport`] is the local adapter.
//! Durable Redis frames use [`WireEnvelope`] + [`RedisTransport`] (feature `redis`).

mod bus;
mod error;
mod handler;
mod message;
mod middleware;
mod transport;
mod wire;

#[cfg(feature = "redis")]
mod redis;

pub use bus::MessageBus;
pub use error::MessengerError;
pub use handler::{CommandHandler, EventHandler};
pub use message::{Command, Event, Message};
pub use middleware::{
    DispatchContext, DispatchKind, LogSink, LoggingMiddleware, Middleware, ValidateHook,
    ValidationMiddleware,
};
pub use transport::{Envelope, InMemoryTransport, Transport};
pub use wire::WireEnvelope;

#[cfg(feature = "redis")]
pub use redis::{RedisTransport, RedisTransportConfig};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
