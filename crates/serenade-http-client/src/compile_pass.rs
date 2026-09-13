//! DI tag and compile pass for the default outbound HTTP client.

use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, DiError, ServiceDefinition};

use crate::DynHttpClient;

/// Service tag applied to outbound HTTP client definitions.
pub const HTTP_CLIENT_TAG: &str = "http.client";

/// Container id of the default outbound client when registered by the pass.
pub const DEFAULT_HTTP_CLIENT_SERVICE: &str = "http_client";

/// Newtype so client trait objects can be stored in the container.
#[derive(Clone)]
pub struct HttpClientService(pub Arc<dyn DynHttpClient>);

impl HttpClientService {
    /// Sends through the wrapped client.
    ///
    /// # Errors
    ///
    /// Propagates [`crate::HttpClientError`] from the client.
    pub fn send(
        &self,
        request: crate::ClientRequest,
    ) -> impl std::future::Future<Output = Result<crate::ClientResponse, crate::HttpClientError>>
    + Send
    + '_ {
        self.0.send_boxed(request)
    }

    /// Convenience GET.
    ///
    /// # Errors
    ///
    /// Propagates [`Self::send`].
    pub fn get(
        &self,
        url: impl Into<String> + Send,
    ) -> impl std::future::Future<Output = Result<crate::ClientResponse, crate::HttpClientError>>
    + Send
    + '_ {
        self.send(crate::ClientRequest::get(url))
    }

    /// Convenience POST with body bytes.
    ///
    /// # Errors
    ///
    /// Propagates [`Self::send`].
    pub fn post(
        &self,
        url: impl Into<String> + Send,
        body: impl Into<Vec<u8>> + Send,
    ) -> impl std::future::Future<Output = Result<crate::ClientResponse, crate::HttpClientError>>
    + Send
    + '_ {
        self.send(crate::ClientRequest::post(url).body(body))
    }
}

/// Seeds [`DEFAULT_HTTP_CLIENT_SERVICE`] with a [`crate::ReqwestHttpClient`] when missing.
///
/// Requires Cargo feature `reqwest` (on by default). Without that feature the pass is a no-op.
#[derive(Debug, Default)]
pub struct RegisterDefaultHttpClientPass;

impl CompilePass for RegisterDefaultHttpClientPass {
    fn name(&self) -> &'static str {
        "register_default_http_client"
    }

    fn process(&self, builder: &mut ContainerBuilder) -> Result<(), DiError> {
        let has_default = builder
            .definitions()
            .iter()
            .any(|definition| definition.id() == DEFAULT_HTTP_CLIENT_SERVICE);
        if has_default {
            return Ok(());
        }

        #[cfg(feature = "reqwest")]
        // `expect`: default id cannot collide after the has_default guard above.
        builder
            .register(
                ServiceDefinition::new(DEFAULT_HTTP_CLIENT_SERVICE).with_tag(HTTP_CLIENT_TAG),
                |_container| {
                    Ok(Box::new(HttpClientService(
                        Arc::new(crate::ReqwestHttpClient::new()) as Arc<dyn DynHttpClient>,
                    )))
                },
            )
            .expect("default http_client id is unique");

        Ok(())
    }
}
