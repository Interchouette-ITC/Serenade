//! HTTP kernel: middleware pipeline plus exception mapping.

use crate::{
    DefaultExceptionHandler, ExceptionHandler, HttpError, Middleware, Request, RequestHandler,
    Response,
};

/// Runs middleware, then the controller, and maps errors to responses.
///
/// # Examples
///
/// ```
/// use serenade_http::{HttpKernel, Method, Request, Response};
///
/// let kernel = HttpKernel::new(|_request: &mut Request| Ok(Response::text(200, "ok")));
/// let response = kernel.handle(Request::new(Method::Get, "/healthz"));
/// assert_eq!(response.status(), 200);
/// assert_eq!(response.body_str(), Some("ok"));
/// ```
pub struct HttpKernel {
    middleware: Vec<Box<dyn Middleware>>,
    controller: Box<dyn RequestHandler>,
    exceptions: Box<dyn ExceptionHandler>,
}

impl HttpKernel {
    /// Kernel with `controller` and [`DefaultExceptionHandler`]. No middleware.
    #[must_use]
    pub fn new(controller: impl RequestHandler + 'static) -> Self {
        Self {
            middleware: Vec::new(),
            controller: Box::new(controller),
            exceptions: Box::new(DefaultExceptionHandler),
        }
    }

    /// Replaces the exception mapper.
    #[must_use]
    pub fn with_exception_handler(mut self, handler: impl ExceptionHandler + 'static) -> Self {
        self.exceptions = Box::new(handler);
        self
    }

    /// Appends middleware. The first pushed layer is outermost (runs first).
    pub fn push_middleware(&mut self, middleware: impl Middleware + 'static) -> &mut Self {
        self.middleware.push(Box::new(middleware));
        self
    }

    /// Handles `request` and always returns a [`Response`].
    #[must_use]
    pub fn handle(&self, mut request: Request) -> Response {
        match self.dispatch(&mut request) {
            Ok(response) => response,
            Err(error) => self.exceptions.handle(&error),
        }
    }

    fn dispatch(&self, request: &mut Request) -> Result<Response, HttpError> {
        Tail {
            middleware: &self.middleware,
            controller: self.controller.as_ref(),
        }
        .handle(request)
    }
}

struct Tail<'a> {
    middleware: &'a [Box<dyn Middleware>],
    controller: &'a dyn RequestHandler,
}

impl RequestHandler for Tail<'_> {
    fn handle(&self, request: &mut Request) -> Result<Response, HttpError> {
        match self.middleware.split_first() {
            Some((head, rest)) => head.process(
                request,
                &Tail {
                    middleware: rest,
                    controller: self.controller,
                },
            ),
            None => self.controller.handle(request),
        }
    }
}
