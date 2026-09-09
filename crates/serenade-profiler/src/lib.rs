//! Web debug toolbar and per-request profiler (Symfony `web_profiler` analogue).
//!
//! Enable only in development. See `docs-dev/PROFILER.md`.

mod config;
mod data;
mod logs;
mod middleware;
mod query;
mod store;
mod toolbar;
mod ui;

use std::fmt::Write as _;

pub use config::ProfilerConfig;
pub use data::{LogLine, ProfileData, QueryEvent};
pub use logs::{ProfilerLogLayer, install_log_scope, with_profile_scope};
pub use middleware::{AsyncProfilerMiddleware, ProfilerMiddleware};
pub use query::record_query;
pub use store::ProfileStore;
pub use toolbar::inject_toolbar;
pub use ui::{profiler_detail_html, profiler_index_html, try_handle_profiler};

/// Attribute key for the active profile token on a [`serenade_http::Request`].
pub const PROFILER_TOKEN_ATTRIBUTE: &str = "_profiler_token";

/// Default URL prefix for profiler UI routes.
pub const PROFILER_PATH_PREFIX: &str = "/_profiler";

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
thread_local! {
    static FORCE_TOKEN_FALLBACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn generate_token() -> String {
    let mut bytes = [0_u8; 8];
    let force_fallback = {
        #[cfg(test)]
        {
            FORCE_TOKEN_FALLBACK.with(std::cell::Cell::get)
        }
        #[cfg(not(test))]
        {
            false
        }
    };
    if force_fallback || getrandom::fill(&mut bytes).is_err() {
        bytes = fallback_token_bytes();
    }
    bytes_to_hex(bytes)
}

fn fallback_token_bytes() -> [u8; 8] {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    #[allow(clippy::cast_possible_truncation)]
    {
        (nanos as u64).to_le_bytes()
    }
}

fn bytes_to_hex(bytes: [u8; 8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests;
