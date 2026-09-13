//! [`HttpClient`] trait.

use crate::{ClientRequest, ClientResponse, HttpClientError};

/// Outbound HTTP client (Symfony `HttpClientInterface` analogue).
pub trait HttpClient: Send + Sync {
    /// Sends `request` and returns the response.
    ///
    /// # Errors
    ///
    /// Returns [`HttpClientError`] on transport or construction failure.
    fn send(
        &self,
        request: ClientRequest,
    ) -> impl std::future::Future<Output = Result<ClientResponse, HttpClientError>> + Send;

    /// Convenience GET.
    ///
    /// # Errors
    ///
    /// Propagates [`Self::send`].
    fn get(
        &self,
        url: impl Into<String> + Send,
    ) -> impl std::future::Future<Output = Result<ClientResponse, HttpClientError>> + Send {
        async move { self.send(ClientRequest::get(url)).await }
    }

    /// Convenience POST with optional body bytes.
    ///
    /// # Errors
    ///
    /// Propagates [`Self::send`].
    fn post(
        &self,
        url: impl Into<String> + Send,
        body: impl Into<Vec<u8>> + Send,
    ) -> impl std::future::Future<Output = Result<ClientResponse, HttpClientError>> + Send {
        async move { self.send(ClientRequest::post(url).body(body)).await }
    }
}
