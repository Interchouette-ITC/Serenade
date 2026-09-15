//! Convert between HTTP/Axum types and [`serenade_http`] types.

use std::str::FromStr;

use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::Response as AxumResponse;
use serenade_http::{
    DefaultExceptionHandler, ExceptionHandler, HttpError, Method, Request, Response,
};

/// Builds a Serenade [`Request`] from HTTP request parts and body bytes.
///
/// Path is `uri.path()`. The raw query string (without `?`) is copied when present.
/// Unsupported methods become [`HttpError`] with status 405.
///
/// # Errors
///
/// Returns [`HttpError`] when the method is not supported by Serenade.
pub fn from_axum(parts: &Parts, body: impl AsRef<[u8]>) -> Result<Request, HttpError> {
    let method = Method::from_str(parts.method.as_str())?;
    let mut serenade = Request::new(method, parts.uri.path()).with_body(body.as_ref().to_vec());
    if let Some(query) = parts.uri.query() {
        serenade = serenade.with_query(query);
    }
    for (name, value) in &parts.headers {
        if let Ok(text) = value.to_str() {
            serenade.headers_mut().insert(name.as_str(), text);
        }
    }
    Ok(serenade)
}

/// Builds an Axum [`Response`](AxumResponse) from a Serenade [`Response`].
#[must_use]
pub fn to_axum(response: &Response) -> AxumResponse {
    let status =
        StatusCode::from_u16(response.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = AxumResponse::builder().status(status);
    for (name, value) in response.headers().iter() {
        let Ok(header_name) = HeaderName::try_from(name) else {
            continue;
        };
        let Ok(header_value) = HeaderValue::from_str(value) else {
            continue;
        };
        builder = builder.header(header_name, header_value);
    }
    builder
        .body(Body::from(response.body().to_vec()))
        .unwrap_or_else(|_| {
            let mut fallback = AxumResponse::new(Body::from("internal error"));
            *fallback.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            fallback
        })
}

/// Maps conversion failure through [`DefaultExceptionHandler`].
#[must_use]
pub fn conversion_error(error: &HttpError) -> AxumResponse {
    to_axum(&DefaultExceptionHandler.handle(error))
}
