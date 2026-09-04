//! HTTP request helper against a foundation [`HttpKernel`](serenade_http::HttpKernel).

use serenade_http::{HttpKernel, Method, Request, Response};

/// Builds requests and runs them through an [`HttpKernel`].
///
/// # Examples
///
/// ```
/// use serenade_http::{HttpKernel, Method, Request, Response};
/// use serenade_testing::HttpTestClient;
///
/// let kernel = HttpKernel::new(|_request: &mut Request| Ok(Response::text(200, "ok")));
/// let client = HttpTestClient::new(&kernel);
/// let response = client.get("/healthz");
/// assert_eq!(response.status(), 200);
/// assert_eq!(response.body_str(), Some("ok"));
/// ```
pub struct HttpTestClient<'a> {
    kernel: &'a HttpKernel,
}

impl<'a> HttpTestClient<'a> {
    /// Client bound to `kernel`.
    #[must_use]
    pub const fn new(kernel: &'a HttpKernel) -> Self {
        Self { kernel }
    }

    /// `GET` request with empty body.
    #[must_use]
    pub fn get(&self, path: &str) -> Response {
        self.request(Method::Get, path, &[])
    }

    /// `POST` request with raw body bytes.
    #[must_use]
    pub fn post(&self, path: &str, body: &[u8]) -> Response {
        self.request(Method::Post, path, body)
    }

    /// Arbitrary method with body.
    #[must_use]
    pub fn request(&self, method: Method, path: &str, body: &[u8]) -> Response {
        let request = Request::new(method, path).with_body(body.to_vec());
        self.kernel.handle(request)
    }
}
