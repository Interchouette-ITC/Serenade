//! Outbound HTTP client contract (Symfony `HttpClient` shaped).
//!
//! - [`HttpClient`]: send a [`ClientRequest`], receive a [`ClientResponse`]
//! - [`MockHttpClient`]: scripted responses for unit tests
//! - [`ReqwestHttpClient`] (feature `reqwest`): default production adapter

mod client;
mod error;
mod mock;
mod request;
mod response;

#[cfg(feature = "reqwest")]
mod reqwest_client;

pub use client::HttpClient;
pub use error::HttpClientError;
pub use mock::MockHttpClient;
pub use request::{ClientMethod, ClientRequest};
pub use response::ClientResponse;

#[cfg(feature = "reqwest")]
pub use reqwest_client::ReqwestHttpClient;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
