//! Incoming HTTP request.

use crate::{AttributeBag, Headers, Method};

/// Framework-agnostic HTTP request.
///
/// Adapters copy method, path, headers, and body from the server crate.
#[derive(Debug)]
pub struct Request {
    method: Method,
    path: String,
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

    /// HTTP method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Path (no query string). Query parsing is left to routing or the adapter.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Request headers.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Mutable headers (adapters and middleware).
    pub fn headers_mut(&mut self) -> &mut Headers {
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
    pub fn attributes_mut(&mut self) -> &mut AttributeBag {
        &mut self.attributes
    }
}
