//! Request-id generate / propagate middleware.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    AsyncMiddleware, AsyncNext, BoxFuture, HttpError, Middleware, Request, RequestHandler, Response,
};

/// Default inbound/outbound header for correlation ids.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Request attribute key storing the resolved request id (`String`).
pub const REQUEST_ID_ATTRIBUTE: &str = "_serenade_request_id";

/// Ensures a request id exists on the request (header + attribute) and echoes it on the response.
#[derive(Debug, Default, Clone, Copy)]
pub struct RequestIdMiddleware;

impl Middleware for RequestIdMiddleware {
    fn process(
        &self,
        request: &mut Request,
        next: &dyn RequestHandler,
    ) -> Result<Response, HttpError> {
        let id = ensure_request_id(request);
        let mut response = next.handle(request)?;
        response.headers_mut().insert(REQUEST_ID_HEADER, id);
        Ok(response)
    }
}

/// Async variant of [`RequestIdMiddleware`].
#[derive(Debug, Default, Clone, Copy)]
pub struct AsyncRequestIdMiddleware;

impl AsyncMiddleware for AsyncRequestIdMiddleware {
    fn process<'a>(
        &'a self,
        request: &'a mut Request,
        next: AsyncNext<'a>,
    ) -> BoxFuture<'a, Result<Response, HttpError>> {
        Box::pin(async move {
            let id = ensure_request_id(request);
            let mut response = next.run(request).await?;
            response.headers_mut().insert(REQUEST_ID_HEADER, id);
            Ok(response)
        })
    }
}

/// Resolves or creates a request id, stores it on the request, and returns a clone.
pub fn ensure_request_id(request: &mut Request) -> String {
    if let Some(existing) = request_id(request) {
        return existing.to_owned();
    }
    let id = match request.headers().get(REQUEST_ID_HEADER) {
        Some(value) if !value.trim().is_empty() => value.trim().to_owned(),
        _ => generate_request_id(),
    };
    request.headers_mut().insert(REQUEST_ID_HEADER, id.clone());
    request
        .attributes_mut()
        .insert(REQUEST_ID_ATTRIBUTE, id.clone());
    id
}

/// Returns the request id from attributes when present.
#[must_use]
pub fn request_id(request: &Request) -> Option<&str> {
    request
        .attributes()
        .get::<String>(REQUEST_ID_ATTRIBUTE)
        .map(String::as_str)
}

fn generate_request_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        });
    format!("{millis:x}-{seq:x}")
}
