//! Dispatch helpers that run Serenade kernels from Axum.

use axum::http::request::Parts;
use axum::response::Response as AxumResponse;
use serenade_http::{AsyncHttpKernel, HttpKernel};

use crate::{conversion_error, from_axum, to_axum};

/// Converts the HTTP request, runs sync `kernel`, and returns an Axum response.
#[must_use]
pub fn dispatch(kernel: &HttpKernel, parts: &Parts, body: impl AsRef<[u8]>) -> AxumResponse {
    match from_axum(parts, body) {
        Ok(serenade) => to_axum(&kernel.handle(serenade)),
        Err(error) => conversion_error(&error),
    }
}

/// Converts the HTTP request, awaits async `kernel`, and returns an Axum response.
#[must_use]
pub async fn dispatch_async(
    kernel: &AsyncHttpKernel,
    parts: &Parts,
    body: impl AsRef<[u8]>,
) -> AxumResponse {
    match from_axum(parts, body) {
        Ok(serenade) => to_axum(&kernel.handle(serenade).await),
        Err(error) => conversion_error(&error),
    }
}
