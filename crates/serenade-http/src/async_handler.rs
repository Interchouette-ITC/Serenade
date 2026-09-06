//! Async controller / innermost handler.

use std::future::Future;
use std::pin::Pin;

use crate::{HttpError, Request, RequestHandler, Response};

/// Owned future returned by [`AsyncRequestHandler::handle`].
pub type BoxFuture<'req, T> = Pin<Box<dyn Future<Output = T> + Send + 'req>>;

/// Async controller: turns a request into a response.
pub trait AsyncRequestHandler: Send + Sync {
    /// Handles `request`.
    ///
    /// # Errors
    ///
    /// Returns [`HttpError`] when the handler cannot produce a response.
    fn handle<'req>(
        &'req self,
        request: &'req mut Request,
    ) -> BoxFuture<'req, Result<Response, HttpError>>;
}

/// Wraps a sync [`RequestHandler`] for use with [`crate::AsyncHttpKernel`].
pub struct SyncToAsync<H>(pub H);

impl<H> AsyncRequestHandler for SyncToAsync<H>
where
    H: RequestHandler,
{
    fn handle<'req>(
        &'req self,
        request: &'req mut Request,
    ) -> BoxFuture<'req, Result<Response, HttpError>> {
        Box::pin(async move { self.0.handle(request) })
    }
}

/// Function-style async controller.
pub struct AsyncFn<F>(pub F);

impl<F> AsyncRequestHandler for AsyncFn<F>
where
    F: for<'req> Fn(&'req mut Request) -> BoxFuture<'req, Result<Response, HttpError>>
        + Send
        + Sync,
{
    fn handle<'req>(
        &'req self,
        request: &'req mut Request,
    ) -> BoxFuture<'req, Result<Response, HttpError>> {
        (self.0)(request)
    }
}

/// Helper to box an async block as [`BoxFuture`].
pub fn box_future<'fut, F>(future: F) -> BoxFuture<'fut, F::Output>
where
    F: Future + Send + 'fut,
{
    Box::pin(future)
}
