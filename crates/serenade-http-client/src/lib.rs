//! Outbound HTTP client contract (Symfony `HttpClient` shaped).
//!
//! - [`HttpClient`]: send a [`ClientRequest`], receive a [`ClientResponse`]
//! - [`DynHttpClient`]: object-safe form for DI (`Arc<dyn DynHttpClient>`)
//! - [`MockHttpClient`]: scripted responses for unit tests
//! - [`ReqwestHttpClient`] (feature `reqwest`): default production adapter
//! - [`RegisterDefaultHttpClientPass`]: DI default service id [`DEFAULT_HTTP_CLIENT_SERVICE`]

mod client;
mod compile_pass;
mod dyn_client;
mod error;
mod mock;
mod request;
mod response;

#[cfg(feature = "reqwest")]
mod reqwest_client;

pub use client::HttpClient;
pub use compile_pass::{
    DEFAULT_HTTP_CLIENT_SERVICE, HTTP_CLIENT_TAG, HttpClientService, RegisterDefaultHttpClientPass,
};
pub use dyn_client::DynHttpClient;
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
