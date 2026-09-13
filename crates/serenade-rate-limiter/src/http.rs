//! HTTP helpers for rejected rate limits.

use serenade_http::Response;

use crate::RateLimit;

/// Builds a `429 Too Many Requests` response from a rejected [`RateLimit`].
///
/// Sets `Retry-After` (seconds, minimum 1 when present), `X-RateLimit-Limit`,
/// and `X-RateLimit-Remaining`.
#[must_use]
pub fn too_many_requests(rate_limit: &RateLimit) -> Response {
    let mut response = Response::text(429, "Too Many Requests")
        .with_header("x-ratelimit-limit", rate_limit.limit().to_string())
        .with_header(
            "x-ratelimit-remaining",
            rate_limit.remaining_tokens().to_string(),
        );
    if let Some(retry_after) = rate_limit.retry_after() {
        let secs = retry_after.as_secs().max(1);
        response = response.with_header("retry-after", secs.to_string());
    }
    response
}
