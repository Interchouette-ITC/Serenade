//! Ops helpers: request-id, health/readiness, shutdown coordination.

mod health;
mod request_id;
mod shutdown;

pub use health::{
    AsyncHealthMiddleware, HEALTHZ_PATH, HealthMiddleware, READYZ_PATH, healthz, readyz,
};
pub use request_id::{
    AsyncRequestIdMiddleware, REQUEST_ID_ATTRIBUTE, REQUEST_ID_HEADER, RequestIdMiddleware,
    ensure_request_id, request_id,
};
pub use shutdown::Readiness;
