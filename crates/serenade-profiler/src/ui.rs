//! Profiler HTML UI helpers.

#![allow(clippy::format_push_string)]

use std::sync::Arc;

use serenade_http::{Method, Request, Response};

use crate::data::ProfileData;
use crate::store::ProfileStore;

/// Handles `/_profiler` and `/_profiler/{token}` when the path matches `prefix`.
///
/// Returns `None` when the request is not a profiler UI route.
#[must_use]
pub fn try_handle_profiler(
    store: &Arc<ProfileStore>,
    prefix: &str,
    request: &Request,
) -> Option<Response> {
    if request.method() != Method::Get {
        return None;
    }
    let path = request.path();
    let prefix = prefix.trim_end_matches('/');
    if path == prefix || path == format!("{prefix}/") {
        return Some(
            Response::new(200)
                .with_header("content-type", "text/html; charset=utf-8")
                .with_body(profiler_index_html(store, prefix).into_bytes()),
        );
    }
    let token_prefix = format!("{prefix}/");
    if let Some(token) = path.strip_prefix(&token_prefix) {
        if token.contains('/') {
            return None;
        }
        return Some(store.get(token).map_or_else(
            || Response::text(404, format!("unknown profiler token `{token}`")),
            |profile| {
                Response::new(200)
                    .with_header("content-type", "text/html; charset=utf-8")
                    .with_body(profiler_detail_html(&profile, prefix).into_bytes())
            },
        ));
    }
    None
}

/// Index page listing recent profiles.
#[must_use]
pub fn profiler_index_html(store: &ProfileStore, prefix: &str) -> String {
    let prefix = prefix.trim_end_matches('/');
    let mut rows = String::new();
    for profile in store.list() {
        rows.push_str(&format!(
            r#"<tr><td><a href="{prefix}/{token}">{token}</a></td><td>{method}</td><td>{path}</td><td>{status}</td><td>{ms} ms</td></tr>"#,
            prefix = escape(prefix),
            token = escape(&profile.token),
            method = escape(&profile.method),
            path = escape(&profile.path),
            status = profile.status,
            ms = profile.duration.as_millis(),
        ));
    }
    if rows.is_empty() {
        rows.push_str(r#"<tr><td colspan="5">No profiles yet.</td></tr>"#);
    }
    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Serenade profiler</title>
<style>body{{font:14px/1.4 system-ui,sans-serif;margin:1.5rem;}}table{{border-collapse:collapse;width:100%;}}td,th{{border:1px solid #ccc;padding:6px 8px;text-align:left;}}</style>
</head><body><h1>Profiler</h1><p>Recent requests (newest first).</p>
<table><thead><tr><th>Token</th><th>Method</th><th>Path</th><th>Status</th><th>Time</th></tr></thead><tbody>{rows}</tbody></table>
<p><a href="/">Back to app</a></p></body></html>"#
    )
}

/// Detail page for one profile.
#[must_use]
pub fn profiler_detail_html(profile: &ProfileData, prefix: &str) -> String {
    let prefix = prefix.trim_end_matches('/');
    let route = profile
        .route
        .as_deref()
        .map_or_else(|| "—".to_owned(), escape);
    let mut queries = String::new();
    if profile.queries.is_empty() {
        queries.push_str("<li>No queries recorded.</li>");
    } else {
        for query in &profile.queries {
            queries.push_str(&format!(
                "<li><code>{}</code> · {} ms{}</li>",
                escape(&query.sql),
                query.duration.as_millis(),
                query
                    .binds_summary
                    .as_ref()
                    .map(|b| format!(" · binds: {}", escape(b)))
                    .unwrap_or_default()
            ));
        }
    }
    let mut logs = String::new();
    if profile.logs.is_empty() {
        logs.push_str("<li>No log lines captured.</li>");
    } else {
        for line in &profile.logs {
            logs.push_str(&format!(
                "<li><strong>{}</strong> [{}] {}</li>",
                escape(&line.level),
                escape(&line.target),
                escape(&line.message)
            ));
        }
    }
    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Profile {token}</title>
<style>body{{font:14px/1.4 system-ui,sans-serif;margin:1.5rem;max-width:960px;}}code{{background:#f4f4f4;padding:1px 4px;}}section{{margin:1.25rem 0;}}h2{{margin-bottom:0.4rem;}}</style>
</head><body>
<p><a href="{prefix}">All profiles</a></p>
<h1>Profile {token}</h1>
<section><h2>Request</h2>
<ul>
<li>Method: <code>{method}</code></li>
<li>Path: <code>{path}</code></li>
<li>Status: {status}</li>
<li>Duration: {ms} ms</li>
<li>Route: <code>{route}</code></li>
</ul></section>
<section><h2>Timing</h2><p>{ms} ms wall time inside profiler middleware.</p></section>
<section><h2>Logs</h2><ul>{logs}</ul></section>
<section><h2>Database</h2><ul>{queries}</ul></section>
<section><h2>Views</h2><p>No view collector yet.</p></section>
</body></html>"#,
        prefix = escape(prefix),
        token = escape(&profile.token),
        method = escape(&profile.method),
        path = escape(&profile.path),
        status = profile.status,
        ms = profile.duration.as_millis(),
        route = route,
        logs = logs,
        queries = queries,
    )
}

fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}
