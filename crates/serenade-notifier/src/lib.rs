//! SMS and push notification channels (Symfony Notifier shaped).
//!
//! Types: [`Notification`], [`Channel`], [`SmsMessage`], [`PushMessage`].
//! Transports: [`NullTransport`], [`MemoryTransport`], [`SmsOnlyTransport`].
//! DI: [`RegisterDefaultNotifierPass`] seeds [`DEFAULT_NOTIFIER_SERVICE`].

mod channel;
mod compile_pass;
mod error;
mod memory;
mod notification;
mod null;
mod sms_only;
mod transport;

pub use channel::Channel;
pub use compile_pass::{
    DEFAULT_NOTIFIER_SERVICE, NOTIFIER_TRANSPORT_TAG, NotifierService, RegisterDefaultNotifierPass,
};
pub use error::NotifierError;
pub use memory::MemoryTransport;
pub use notification::{Notification, PushMessage, SmsMessage};
pub use null::NullTransport;
pub use sms_only::SmsOnlyTransport;
pub use transport::Transport;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
