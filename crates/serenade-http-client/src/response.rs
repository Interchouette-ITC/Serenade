//! Outbound response.

use std::collections::BTreeMap;

/// Outbound HTTP response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientResponse {
    status: u16,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

impl ClientResponse {
    /// Builds a response.
    #[must_use]
    pub const fn new(status: u16) -> Self {
        Self {
            status,
            headers: BTreeMap::new(),
            body: Vec::new(),
        }
    }

    /// Sets a header (name stored lowercased).
    #[must_use]
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .insert(name.into().to_ascii_lowercase(), value.into());
        self
    }

    /// Sets the body bytes.
    #[must_use]
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    /// Sets a UTF-8 body.
    #[must_use]
    pub fn body_text(self, text: impl Into<String>) -> Self {
        self.body(text.into().into_bytes())
    }

    /// Status code.
    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    /// True when status is 2xx.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Headers (lowercased names).
    #[must_use]
    pub const fn headers(&self) -> &BTreeMap<String, String> {
        &self.headers
    }

    /// Header value by name (case-insensitive).
    #[must_use]
    pub fn header_value(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    /// Body bytes.
    #[must_use]
    pub fn body_bytes(&self) -> &[u8] {
        &self.body
    }

    /// Body as lossy UTF-8.
    #[must_use]
    pub fn body_text_lossy(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}
