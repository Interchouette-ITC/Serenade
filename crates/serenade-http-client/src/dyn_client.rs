//! Object-safe outbound client for DI (`Arc<dyn DynHttpClient>`).

use std::future::Future;
use std::pin::Pin;

use crate::{ClientRequest, ClientResponse, HttpClient, HttpClientError, MockHttpClient};

/// Object-safe sibling of [`HttpClient`] for container storage.
pub trait DynHttpClient: Send + Sync {
    /// Sends `request` and returns the response.
    fn send_boxed(
        &self,
        request: ClientRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ClientResponse, HttpClientError>> + Send + '_>>;
}

impl DynHttpClient for MockHttpClient {
    fn send_boxed(
        &self,
        request: ClientRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ClientResponse, HttpClientError>> + Send + '_>> {
        Box::pin(HttpClient::send(self, request))
    }
}

#[cfg(feature = "reqwest")]
impl DynHttpClient for crate::ReqwestHttpClient {
    fn send_boxed(
        &self,
        request: ClientRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ClientResponse, HttpClientError>> + Send + '_>> {
        Box::pin(HttpClient::send(self, request))
    }
}
