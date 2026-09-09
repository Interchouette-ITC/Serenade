//! Profile payload types.

use std::time::Duration;

/// One SQL (or other) statement reported by an app adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryEvent {
    /// Statement text (apps should redact secrets).
    pub sql: String,
    /// Execution duration.
    pub duration: Duration,
    /// Optional bind / parameter summary (no raw secrets).
    pub binds_summary: Option<String>,
}

impl QueryEvent {
    /// Builds a query event.
    #[must_use]
    pub fn new(sql: impl Into<String>, duration: Duration) -> Self {
        Self {
            sql: sql.into(),
            duration,
            binds_summary: None,
        }
    }

    /// Adds a bind summary.
    #[must_use]
    pub fn with_binds(mut self, summary: impl Into<String>) -> Self {
        self.binds_summary = Some(summary.into());
        self
    }
}

/// One captured log line for the request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogLine {
    /// tracing target / channel.
    pub target: String,
    /// Level label (`INFO`, …).
    pub level: String,
    /// Formatted message.
    pub message: String,
}

/// Collected data for one HTTP request.
#[derive(Clone, Debug)]
pub struct ProfileData {
    /// Opaque token used in `/_profiler/{token}`.
    pub token: String,
    /// HTTP method string.
    pub method: String,
    /// Request path.
    pub path: String,
    /// Response status code.
    pub status: u16,
    /// Wall time for the middleware span.
    pub duration: Duration,
    /// Matched route name when present.
    pub route: Option<String>,
    /// Query events pushed by adapters.
    pub queries: Vec<QueryEvent>,
    /// Log lines captured while the request scope was active.
    pub logs: Vec<LogLine>,
}

impl ProfileData {
    pub(crate) const fn new(token: String, method: String, path: String) -> Self {
        Self {
            token,
            method,
            path,
            status: 0,
            duration: Duration::ZERO,
            route: None,
            queries: Vec::new(),
            logs: Vec::new(),
        }
    }
}
