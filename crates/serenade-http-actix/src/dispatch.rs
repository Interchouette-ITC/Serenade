//! Dispatch helpers that run Serenade kernels from Actix.

use actix_web::{HttpRequest, HttpResponse};
use serenade_http::{AsyncHttpKernel, HttpKernel};

use crate::{conversion_error, from_actix, to_actix};

/// Converts the Actix request, runs sync `kernel`, and returns an Actix response.
#[must_use]
pub fn dispatch(
    kernel: &HttpKernel,
    request: &HttpRequest,
    body: impl AsRef<[u8]>,
) -> HttpResponse {
    match from_actix(request, body) {
        Ok(serenade) => to_actix(&kernel.handle(serenade)),
        Err(error) => conversion_error(&error),
    }
}

/// Converts the Actix request, awaits async `kernel`, and returns an Actix response.
///
/// Actix `HttpRequest` is `!Send`; clippy nursery flags the resulting future.
#[must_use]
#[allow(clippy::future_not_send)]
pub async fn dispatch_async(
    kernel: &AsyncHttpKernel,
    request: &HttpRequest,
    body: impl AsRef<[u8]>,
) -> HttpResponse {
    match from_actix(request, body) {
        Ok(serenade) => to_actix(&kernel.handle(serenade).await),
        Err(error) => conversion_error(&error),
    }
}
