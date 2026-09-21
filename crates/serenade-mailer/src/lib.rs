//! Email message, Mime types, and sync transports (Symfony Mailer shaped).
//!
//! Types: [`Email`], [`Address`], [`Body`], [`Attachment`], [`MimeTree`].
//! Transports: [`NullTransport`], [`FileTransport`], and (feature `smtp`) [`SmtpTransport`].
//! Feature `esp` adds `EspHttpTransport` (SendGrid-class HTTP mail API).
//! DI: [`RegisterDefaultMailerPass`] seeds service id [`DEFAULT_MAILER_SERVICE`].

mod address;
mod attachment;
mod body;
mod compile_pass;
mod email;
mod error;
mod file;
mod mime;
mod null;
mod render;
mod transport;

#[cfg(feature = "esp")]
mod esp;
#[cfg(feature = "smtp")]
mod smtp;

pub use address::Address;
pub use attachment::{Attachment, ContentDisposition};
pub use body::Body;
pub use compile_pass::{
    DEFAULT_MAILER_SERVICE, MAILER_TRANSPORT_TAG, MailerService, RegisterDefaultMailerPass,
};
pub use email::Email;
pub use error::MailerError;
pub use file::FileTransport;
pub use mime::{MimePart, MimeTree};
pub use null::NullTransport;
pub use transport::Transport;

#[cfg(feature = "esp")]
pub use esp::{
    EspApiConfig, EspAuthScheme, EspHttpPoster, EspHttpResponse, EspHttpTransport,
    MockEspHttpPoster, MockEspRequest, build_esp_payload,
};
#[cfg(feature = "smtp")]
pub use smtp::{SmtpTransport, SmtpTransportBuilder};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
