//! Dispatch helpers that run [`serenade_http::HttpKernel`] from Actix.

use actix_web::{HttpRequest, HttpResponse};
use serenade_http::HttpKernel;

use crate::{conversion_error, from_actix, to_actix};

/// Converts the Actix request, runs `kernel`, and returns an Actix response.
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
