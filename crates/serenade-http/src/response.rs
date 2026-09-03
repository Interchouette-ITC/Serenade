//! Outgoing HTTP response.

use crate::Headers;

/// Framework-agnostic HTTP response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Response {
    status: u16,
    headers: Headers,
    body: Vec<u8>,
}

impl Response {
    /// Empty body at `status`.
    #[must_use]
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Headers::new(),
            body: Vec::new(),
        }
    }

    /// UTF-8 body with `text/plain; charset=utf-8`.
    #[must_use]
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self::new(status)
            .with_header("content-type", "text/plain; charset=utf-8")
            .with_body(body.into().into_bytes())
    }

    /// Adds or replaces a header.
    #[must_use]
    pub fn with_header(mut self, name: impl AsRef<str>, value: impl Into<String>) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Sets the raw body.
    #[must_use]
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    /// Status code.
    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    /// Response headers.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Raw body bytes.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Body as UTF-8 when valid.
    #[must_use]
    pub fn body_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.body).ok()
    }
}
