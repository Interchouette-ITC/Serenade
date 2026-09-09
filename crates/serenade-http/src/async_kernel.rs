//! Async HTTP kernel: middleware pipeline plus exception mapping.

use crate::{
    AsyncMiddleware, AsyncNext, AsyncRequestHandler, BoxFuture, DefaultExceptionHandler,
    ExceptionHandler, HttpError, Request, Response, SyncToAsync,
};

/// Runs async middleware, then the controller, and maps errors to responses.
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
    middleware: Vec<Box<dyn AsyncMiddleware>>,
    controller: Box<dyn AsyncRequestHandler>,
    exceptions: Box<dyn ExceptionHandler>,
}

impl AsyncHttpKernel {
    /// Kernel with async `controller` and [`DefaultExceptionHandler`].
    #[must_use]
    pub fn new(controller: impl AsyncRequestHandler + 'static) -> Self {
        Self {
            middleware: Vec::new(),
            controller: Box::new(controller),
            exceptions: Box::new(DefaultExceptionHandler),
        }
    }

    /// Builds a kernel from an async function returning [`BoxFuture`].
    #[must_use]
    pub fn from_async_fn<F>(handler: F) -> Self
    where
        F: for<'req> Fn(&'req mut Request) -> BoxFuture<'req, Result<Response, HttpError>>
            + Send
            + Sync
            + 'static,
    {
        Self::new(crate::AsyncFn(handler))
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

    /// Appends middleware. The first pushed layer is outermost (runs first).
    pub fn push_middleware(&mut self, middleware: impl AsyncMiddleware + 'static) -> &mut Self {
        self.middleware.push(Box::new(middleware));
        self
    }

    /// Handles `request` and always returns a [`Response`].
    #[must_use = "futures do nothing unless you await them"]
    pub fn handle(&self, mut request: Request) -> BoxFuture<'_, Response> {
        Box::pin(async move {
            match self.dispatch(&mut request).await {
                Ok(response) => response,
                Err(error) => self.exceptions.handle(&error),
            }
        })
    }

    fn dispatch<'req>(
        &'req self,
        request: &'req mut Request,
    ) -> BoxFuture<'req, Result<Response, HttpError>> {
        AsyncNext {
            middleware: &self.middleware,
            controller: self.controller.as_ref(),
        }
        .run(request)
    }
}
