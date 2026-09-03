//! Maps [`crate::HttpError`] to a [`crate::Response`].

use crate::{HttpError, Response};

/// Hook invoked when a handler or middleware returns [`HttpError`].
pub trait ExceptionHandler: Send + Sync {
    /// Builds a response for `error`.
    fn handle(&self, error: &HttpError) -> Response;
}

/// Maps [`HttpError::status_code`] and [`HttpError::message`] to a text response.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultExceptionHandler;

impl ExceptionHandler for DefaultExceptionHandler {
    fn handle(&self, error: &HttpError) -> Response {
        Response::text(error.status_code(), error.message())
    }
}
