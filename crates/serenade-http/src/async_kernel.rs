//! Async HTTP kernel: controller pipeline with exception mapping.

use crate::{
    AsyncFn, AsyncRequestHandler, BoxFuture, DefaultExceptionHandler, ExceptionHandler, HttpError,
    Request, Response, SyncToAsync,
};

/// Runs an async controller and maps errors to responses.
///
/// Middleware is not wired on this type yet; use for Actix/`listen` apps that
/// need `await` in controllers (database I/O). Sync stacks keep using
/// [`crate::HttpKernel`].
///
/// # Examples
///
/// ```
/// use serenade_http::{AsyncHttpKernel, Request, Response};
///
/// let _kernel = AsyncHttpKernel::from_sync(|_request: &mut Request| {
///     Ok(Response::text(200, "ok"))
/// });
/// ```
pub struct AsyncHttpKernel {
    controller: Box<dyn AsyncRequestHandler>,
    exceptions: Box<dyn ExceptionHandler>,
}

impl AsyncHttpKernel {
    /// Kernel with async `controller` and [`DefaultExceptionHandler`].
    #[must_use]
    pub fn new(controller: impl AsyncRequestHandler + 'static) -> Self {
        Self {
            controller: Box::new(controller),
            exceptions: Box::new(DefaultExceptionHandler),
        }
    }

    /// Builds a kernel from an async function returning [`BoxFuture`].
    #[must_use]
    pub fn from_async_fn<F>(handler: F) -> Self
    where
        F: for<'a> Fn(&'a mut Request) -> BoxFuture<'a, Result<Response, HttpError>>
            + Send
            + Sync
            + 'static,
    {
        Self::new(AsyncFn(handler))
    }

    /// Wraps a sync [`crate::RequestHandler`] as an async kernel.
    #[must_use]
    pub fn from_sync(controller: impl crate::RequestHandler + 'static) -> Self {
        Self::new(SyncToAsync(controller))
    }

    /// Replaces the exception mapper.
    #[must_use]
    pub fn with_exception_handler(mut self, handler: impl ExceptionHandler + 'static) -> Self {
        self.exceptions = Box::new(handler);
        self
    }

    /// Handles `request` and always returns a [`Response`].
    pub fn handle(&self, mut request: Request) -> BoxFuture<'_, Response> {
        Box::pin(async move {
            match self.dispatch(&mut request).await {
                Ok(response) => response,
                Err(error) => self.exceptions.handle(&error),
            }
        })
    }

    fn dispatch<'a>(
        &'a self,
        request: &'a mut Request,
    ) -> BoxFuture<'a, Result<Response, HttpError>> {
        self.controller.handle(request)
    }
}
