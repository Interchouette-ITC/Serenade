//! Incoming HTTP request.

use crate::{AttributeBag, Headers, Method};

/// Framework-agnostic HTTP request.
///
/// Adapters copy method, path, optional query string, headers, and body from the
/// server crate.
#[derive(Debug)]
pub struct Request {
    method: Method,
    path: String,
    query: Option<String>,
    headers: Headers,
    body: Vec<u8>,
    attributes: AttributeBag,
}

impl Request {
    /// Builds a request with an empty body, headers, and attributes.
    #[must_use]
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            query: None,
            headers: Headers::new(),
            body: Vec::new(),
            attributes: AttributeBag::new(),
        }
    }

    /// Replaces the header map after insert.
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

    /// Sets the raw query string without a leading `?`.
    #[must_use]
    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        let query = query.into();
        self.query = if query.is_empty() { None } else { Some(query) };
        self
    }

    /// HTTP method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Path (no query string).
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Raw query string without a leading `?`, when the adapter provided one.
    #[must_use]
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    /// Request headers.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Mutable headers (adapters and middleware).
    pub const fn headers_mut(&mut self) -> &mut Headers {
        &mut self.headers
    }

    /// Raw body bytes.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Request attributes.
    #[must_use]
    pub const fn attributes(&self) -> &AttributeBag {
        &self.attributes
    }

    /// Mutable attributes (middleware, router, controller).
    pub const fn attributes_mut(&mut self) -> &mut AttributeBag {
        &mut self.attributes
    }
}
