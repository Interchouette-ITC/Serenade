//! Readiness flag for probes and graceful drain.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Shared readiness bit flipped false before HTTP drain / process exit.
///
/// Wire into [`super::HealthMiddleware`] / [`super::AsyncHealthMiddleware`]. Typical
/// stop order: `mark_not_ready`, stop the HTTP server (Actix `ServerHandle` /
/// Axum shutdown handle), then run application kernel shutdown.
#[derive(Clone, Debug, Default)]
pub struct Readiness {
    ready: Arc<AtomicBool>,
}

impl Readiness {
    /// Starts ready (`true`).
    #[must_use]
    pub fn new() -> Self {
        Self {
            ready: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Whether probes should report ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }

    /// Marks the process not ready (fail readiness probes while draining).
    pub fn mark_not_ready(&self) {
        self.ready.store(false, Ordering::SeqCst);
    }

    /// Marks the process ready again (tests / rare re-arm).
    pub fn mark_ready(&self) {
        self.ready.store(true, Ordering::SeqCst);
    }
}
