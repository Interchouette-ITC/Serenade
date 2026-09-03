//! Controller / innermost handler.

use crate::{HttpError, Request, Response};

/// Turns a request into a response or an error for the exception mapper.
pub trait RequestHandler: Send + Sync {
    /// Handles `request`. Middleware may mutate attributes before this runs.
    ///
    /// # Errors
    ///
    /// Returns [`HttpError`] when the handler cannot produce a response.
    fn handle(&self, request: &mut Request) -> Result<Response, HttpError>;
}

impl<F> RequestHandler for F
where
    F: Fn(&mut Request) -> Result<Response, HttpError> + Send + Sync,
{
    fn handle(&self, request: &mut Request) -> Result<Response, HttpError> {
        self(request)
    }
}
