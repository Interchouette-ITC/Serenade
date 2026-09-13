//! Reqwest-backed [`HttpClient`].

use std::time::Duration;

use reqwest::Client;

use crate::{ClientMethod, ClientRequest, ClientResponse, HttpClient, HttpClientError};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Production [`HttpClient`] using reqwest + rustls.
#[derive(Clone, Debug)]
pub struct ReqwestHttpClient {
    client: Client,
    default_timeout: Duration,
}

impl ReqwestHttpClient {
    /// Builds a client with a 30s default timeout.
    ///
    /// # Panics
    ///
    /// Panics when the underlying reqwest client cannot be built (should not happen
    /// for a timeout-only builder).
    #[must_use]
    pub fn new() -> Self {
        Self::with_timeout(DEFAULT_TIMEOUT)
    }

    /// Builds a client with `default_timeout` applied when a request has no timeout.
    ///
    /// # Panics
    ///
    /// Panics when the underlying reqwest client cannot be built (should not happen
    /// for a timeout-only builder).
    #[must_use]
    pub fn with_timeout(default_timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(default_timeout)
            .build()
            .expect("reqwest client");
        Self {
            client,
            default_timeout,
        }
    }

    /// Default timeout used when [`ClientRequest::timeout_duration`] is unset.
    #[must_use]
    pub const fn default_timeout(&self) -> Duration {
        self.default_timeout
    }
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for ReqwestHttpClient {
    async fn send(&self, request: ClientRequest) -> Result<ClientResponse, HttpClientError> {
        let method = map_method(request.method());
        let mut builder = self.client.request(method, request.url());
        for (name, value) in request.headers() {
            builder = builder.header(name.as_str(), value.as_str());
        }
        if !request.body_bytes().is_empty() {
            builder = builder.body(request.body_bytes().to_vec());
        }
        if let Some(timeout) = request.timeout_duration() {
            builder = builder.timeout(timeout);
        }
        let response = builder
            .send()
            .await
            .map_err(|err| HttpClientError::Transport {
                message: err.to_string(),
            })?;
        let status = response.status().as_u16();
        let mut headers = std::collections::BTreeMap::new();
        for (name, value) in response.headers() {
            if let Ok(text) = value.to_str() {
                headers.insert(name.as_str().to_ascii_lowercase(), text.to_owned());
            }
        }
        let body = response.bytes().await.expect("response body");
        let mut out = ClientResponse::new(status).body(body.to_vec());
        for (name, value) in headers {
            out = out.header(name, value);
        }
        Ok(out)
    }
}

const fn map_method(method: ClientMethod) -> reqwest::Method {
    match method {
        ClientMethod::Get => reqwest::Method::GET,
        ClientMethod::Post => reqwest::Method::POST,
        ClientMethod::Put => reqwest::Method::PUT,
        ClientMethod::Patch => reqwest::Method::PATCH,
        ClientMethod::Delete => reqwest::Method::DELETE,
        ClientMethod::Head => reqwest::Method::HEAD,
    }
}
