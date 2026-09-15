//! Liveness / readiness helpers and short-circuit middleware.

use crate::ops::Readiness;
use crate::{
    AsyncMiddleware, AsyncNext, BoxFuture, HttpError, Middleware, Request, RequestHandler, Response,
};

/// Default liveness probe path.
pub const HEALTHZ_PATH: &str = "/healthz";

/// Default readiness probe path.
pub const READYZ_PATH: &str = "/readyz";

/// Liveness response (`200` + `ok`).
#[must_use]
pub fn healthz() -> Response {
    Response::text(200, "ok")
}

/// Readiness response: `200` + `ready`, or `503` + `not ready`.
#[must_use]
pub fn readyz(ready: bool) -> Response {
    if ready {
        Response::text(200, "ready")
    } else {
        Response::text(503, "not ready")
    }
}

/// Short-circuits [`HEALTHZ_PATH`] / [`READYZ_PATH`] before the rest of the pipeline.
pub struct HealthMiddleware {
    readiness: Readiness,
}

impl HealthMiddleware {
    /// Middleware that reports readiness via `readiness`.
    #[must_use]
    pub const fn new(readiness: Readiness) -> Self {
        Self { readiness }
    }

    /// Convenience constructor with an always-ready flag.
    #[must_use]
    pub fn always_ready() -> Self {
        Self::new(Readiness::new())
    }

    /// Shared readiness handle (flip before HTTP drain).
    #[must_use]
    pub const fn readiness(&self) -> &Readiness {
        &self.readiness
    }
}

impl Middleware for HealthMiddleware {
    fn process(
        &self,
        request: &mut Request,
        next: &dyn RequestHandler,
    ) -> Result<Response, HttpError> {
        if let Some(response) = probe_response(request.path(), self.readiness.is_ready()) {
            return Ok(response);
        }
        next.handle(request)
    }
}

/// Async variant of [`HealthMiddleware`].
pub struct AsyncHealthMiddleware {
    readiness: Readiness,
}

impl AsyncHealthMiddleware {
    /// Middleware that reports readiness via `readiness`.
    #[must_use]
    pub const fn new(readiness: Readiness) -> Self {
        Self { readiness }
    }

    /// Convenience constructor with an always-ready flag.
    #[must_use]
    pub fn always_ready() -> Self {
        Self::new(Readiness::new())
    }

    /// Shared readiness handle (flip before HTTP drain).
    #[must_use]
    pub const fn readiness(&self) -> &Readiness {
        &self.readiness
    }
}

impl AsyncMiddleware for AsyncHealthMiddleware {
    fn process<'a>(
        &'a self,
        request: &'a mut Request,
        next: AsyncNext<'a>,
    ) -> BoxFuture<'a, Result<Response, HttpError>> {
        Box::pin(async move {
            if let Some(response) = probe_response(request.path(), self.readiness.is_ready()) {
                return Ok(response);
            }
            next.run(request).await
        })
    }
}

fn probe_response(path: &str, ready: bool) -> Option<Response> {
    match path {
        HEALTHZ_PATH => Some(healthz()),
        READYZ_PATH => Some(readyz(ready)),
        _ => None,
    }
}
