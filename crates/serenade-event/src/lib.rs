//! Event dispatcher for domain and infrastructure events.
//!
//! Subscribers are registered by bundles; dispatch stays synchronous by default.

mod compile_pass;
mod dispatcher;
mod error;
mod harness;
mod subscriber;

pub use compile_pass::{
    DISPATCHER_SERVICE, RegisterEventSubscribersPass, SUBSCRIBER_TAG, SubscriberService,
};
pub use dispatcher::EventDispatcher;
pub use error::EventError;
pub use harness::{RecordingSubscriber, assert_dispatched};
pub use subscriber::{Event, EventSubscriber};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
