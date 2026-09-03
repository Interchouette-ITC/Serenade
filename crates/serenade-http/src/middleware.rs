//! Middleware around [`crate::RequestHandler`].

use crate::{HttpError, Request, RequestHandler, Response};

/// Layer that may inspect or mutate the request, then call `next`.
pub trait Middleware: Send + Sync {
    /// Runs this layer.
    ///
    /// # Errors
    ///
    /// Returns [`HttpError`] instead of calling `next`, or when `next` fails.
    fn process(
        &self,
        request: &mut Request,
        next: &dyn RequestHandler,
    ) -> Result<Response, HttpError>;
}
