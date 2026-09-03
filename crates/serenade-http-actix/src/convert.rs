//! Convert between Actix types and [`serenade_http`] types.

use std::str::FromStr;

use actix_web::http::{header::HeaderValue, StatusCode};
use actix_web::{HttpRequest, HttpResponse};
use serenade_http::{
    DefaultExceptionHandler, ExceptionHandler, HttpError, Method, Request, Response,
};

/// Builds a Serenade [`Request`] from an Actix request and body bytes.
///
/// Path is `HttpRequest::path` (no query string). Unsupported methods become
/// [`HttpError`] with status 405.
///
/// # Errors
///
/// Returns [`HttpError`] when the Actix method is not supported by Serenade.
pub fn from_actix(request: &HttpRequest, body: impl AsRef<[u8]>) -> Result<Request, HttpError> {
    let method = Method::from_str(request.method().as_str())?;
    let mut serenade = Request::new(method, request.path()).with_body(body.as_ref().to_vec());
    for (name, value) in request.headers() {
        if let Ok(text) = value.to_str() {
            serenade.headers_mut().insert(name.as_str(), text);
        }
    }
    Ok(serenade)
}

/// Builds an Actix [`HttpResponse`] from a Serenade [`Response`].
#[must_use]
pub fn to_actix(response: &Response) -> HttpResponse {
    let status =
        StatusCode::from_u16(response.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = HttpResponse::build(status);
    for (name, value) in response.headers().iter() {
        if let Ok(header_value) = HeaderValue::from_str(value) {
            builder.append_header((name, header_value));
        }
    }
    builder.body(response.body().to_vec())
}

/// Maps conversion failure through [`DefaultExceptionHandler`].
#[must_use]
pub fn conversion_error(error: &HttpError) -> HttpResponse {
    to_actix(&DefaultExceptionHandler.handle(error))
}
