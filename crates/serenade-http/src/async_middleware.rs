//! Async middleware around the controller pipeline.

use crate::{AsyncRequestHandler, BoxFuture, HttpError, Request, Response};

/// Remainder of the async middleware / controller stack.
pub struct AsyncNext<'a> {
    pub(crate) middleware: &'a [Box<dyn AsyncMiddleware>],
    pub(crate) controller: &'a dyn AsyncRequestHandler,
}

impl<'a> AsyncNext<'a> {
    /// Runs the next middleware or the controller.
    pub fn run(self, request: &'a mut Request) -> BoxFuture<'a, Result<Response, HttpError>> {
        match self.middleware.split_first() {
            Some((head, rest)) => head.process(
                request,
                AsyncNext {
                    middleware: rest,
                    controller: self.controller,
                },
            ),
            None => self.controller.handle(request),
        }
    }
}

/// Layer that may inspect or mutate the request, then await [`AsyncNext::run`].
pub trait AsyncMiddleware: Send + Sync {
    /// Runs this layer. Call `next.run(request).await` to continue the pipeline.
    fn process<'a>(
        &'a self,
        request: &'a mut Request,
        next: AsyncNext<'a>,
    ) -> BoxFuture<'a, Result<Response, HttpError>>;
}
