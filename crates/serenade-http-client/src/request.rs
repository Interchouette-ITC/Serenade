//! Outbound request builder.

use std::collections::BTreeMap;
use std::time::Duration;

/// HTTP method for [`ClientRequest`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ClientMethod {
    /// GET.
    Get,
    /// POST.
    Post,
    /// PUT.
    Put,
    /// PATCH.
    Patch,
    /// DELETE.
    Delete,
    /// HEAD.
    Head,
}

impl ClientMethod {
    /// Uppercase method token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
        }
    }
}

impl std::fmt::Display for ClientMethod {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Outbound HTTP request (apps build this; clients send it).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientRequest {
    method: ClientMethod,
    url: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
    timeout: Option<Duration>,
}

impl ClientRequest {
    /// Starts a GET request.
    #[must_use]
    pub fn get(url: impl Into<String>) -> Self {
        Self::new(ClientMethod::Get, url)
    }

    /// Starts a POST request.
    #[must_use]
    pub fn post(url: impl Into<String>) -> Self {
        Self::new(ClientMethod::Post, url)
    }

    /// Starts a PUT request.
    #[must_use]
    pub fn put(url: impl Into<String>) -> Self {
        Self::new(ClientMethod::Put, url)
    }

    /// Starts a PATCH request.
    #[must_use]
    pub fn patch(url: impl Into<String>) -> Self {
        Self::new(ClientMethod::Patch, url)
    }

    /// Starts a DELETE request.
    #[must_use]
    pub fn delete(url: impl Into<String>) -> Self {
        Self::new(ClientMethod::Delete, url)
    }

    /// Starts a HEAD request.
    #[must_use]
    pub fn head(url: impl Into<String>) -> Self {
        Self::new(ClientMethod::Head, url)
    }

    /// Builds a request with method and URL.
    #[must_use]
    pub fn new(method: ClientMethod, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: BTreeMap::new(),
            body: Vec::new(),
            timeout: None,
        }
    }

    /// Sets a header (name stored lowercased).
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .insert(name.into().to_ascii_lowercase(), value.into());
        self
    }

    /// Sets the raw body bytes.
    #[must_use]
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    /// Sets a UTF-8 body and `content-type: text/plain; charset=utf-8` when absent.
    #[must_use]
    pub fn body_text(mut self, text: impl Into<String>) -> Self {
        self.body = text.into().into_bytes();
        self.headers
            .entry("content-type".to_owned())
            .or_insert_with(|| "text/plain; charset=utf-8".to_owned());
        self
    }

    /// Per-request timeout (overrides the client default when set).
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// HTTP method.
    #[must_use]
    pub const fn method(&self) -> ClientMethod {
        self.method
    }

    /// Target URL.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Headers (lowercased names).
    #[must_use]
    pub const fn headers(&self) -> &BTreeMap<String, String> {
        &self.headers
    }

    /// Body bytes.
    #[must_use]
    pub fn body_bytes(&self) -> &[u8] {
        &self.body
    }

    /// Optional timeout.
    #[must_use]
    pub const fn timeout_duration(&self) -> Option<Duration> {
        self.timeout
    }
}
