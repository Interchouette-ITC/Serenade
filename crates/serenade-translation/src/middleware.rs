//! HTTP middleware that stores the negotiated locale on the request.

use serenade_http::{HttpError, Middleware, Request, RequestHandler, Response};

use crate::LocaleNegotiator;

/// Runs [`LocaleNegotiator::apply`] before the next handler.
#[derive(Clone, Debug)]
pub struct LocaleMiddleware {
    negotiator: LocaleNegotiator,
}

impl LocaleMiddleware {
    /// Middleware wrapping `negotiator`.
    #[must_use]
    pub const fn new(negotiator: LocaleNegotiator) -> Self {
        Self { negotiator }
    }
}

impl Middleware for LocaleMiddleware {
    fn process(
        &self,
        request: &mut Request,
        next: &dyn RequestHandler,
    ) -> Result<Response, HttpError> {
        self.negotiator.apply(request);
        next.handle(request)
    }
}
