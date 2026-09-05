//! Async controller / innermost handler.

use std::future::Future;
use std::pin::Pin;

use crate::{HttpError, Request, RequestHandler, Response};

/// Owned future returned by [`AsyncRequestHandler::handle`].
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Async controller: turns a request into a response.
pub trait AsyncRequestHandler: Send + Sync {
    /// Handles `request`.
    ///
    /// # Errors
    ///
    /// Returns [`HttpError`] when the handler cannot produce a response.
    fn handle<'a>(&'a self, request: &'a mut Request)
        -> BoxFuture<'a, Result<Response, HttpError>>;
}

/// Wraps a sync [`RequestHandler`] for use with [`crate::AsyncHttpKernel`].
pub struct SyncToAsync<H>(pub H);

impl<H> AsyncRequestHandler for SyncToAsync<H>
where
    H: RequestHandler,
{
    fn handle<'a>(
        &'a self,
        request: &'a mut Request,
    ) -> BoxFuture<'a, Result<Response, HttpError>> {
        Box::pin(async move { self.0.handle(request) })
    }
}

/// Function-style async controller.
pub struct AsyncFn<F>(pub F);

impl<F> AsyncRequestHandler for AsyncFn<F>
where
    F: for<'a> Fn(&'a mut Request) -> BoxFuture<'a, Result<Response, HttpError>> + Send + Sync,
{
    fn handle<'a>(
        &'a self,
        request: &'a mut Request,
    ) -> BoxFuture<'a, Result<Response, HttpError>> {
        (self.0)(request)
    }
}

/// Helper to box an async block as [`BoxFuture`].
pub fn box_future<'a, F>(future: F) -> BoxFuture<'a, F::Output>
where
    F: Future + Send + 'a,
{
    Box::pin(future)
}
