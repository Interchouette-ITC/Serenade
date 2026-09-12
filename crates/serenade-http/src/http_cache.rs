//! HTTP response cache headers and conditional GET (304).
//!
//! This is **wire** caching (`Cache-Control`, `ETag`, …), not the `serenade-cache`
//! item pool. Apps use both: pools for application data, these helpers for
//! browser/proxy freshness.

use crate::{Method, Request, Response};

/// Cache-related headers to apply on a [`Response`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HttpCacheHeaders {
    cache_control: Option<String>,
    etag: Option<String>,
    last_modified: Option<String>,
    vary: Option<String>,
}

impl HttpCacheHeaders {
    /// Empty policy (no headers until builders set them).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `Cache-Control: public, max-age={seconds}`.
    #[must_use]
    pub fn public_max_age(seconds: u64) -> Self {
        Self::new().with_cache_control(format!("public, max-age={seconds}"))
    }

    /// `Cache-Control: private, max-age={seconds}`.
    #[must_use]
    pub fn private_max_age(seconds: u64) -> Self {
        Self::new().with_cache_control(format!("private, max-age={seconds}"))
    }

    /// `Cache-Control: no-store`.
    #[must_use]
    pub fn no_store() -> Self {
        Self::new().with_cache_control("no-store")
    }

    /// Sets `Cache-Control` to an arbitrary value.
    #[must_use]
    pub fn with_cache_control(mut self, value: impl Into<String>) -> Self {
        self.cache_control = Some(value.into());
        self
    }

    /// Sets `ETag`. Pass an already-quoted value, or use [`etag`] / [`weak_etag`].
    #[must_use]
    pub fn with_etag(mut self, value: impl Into<String>) -> Self {
        self.etag = Some(value.into());
        self
    }

    /// Sets `Last-Modified` (HTTP-date string as produced by the app).
    #[must_use]
    pub fn with_last_modified(mut self, value: impl Into<String>) -> Self {
        self.last_modified = Some(value.into());
        self
    }

    /// Sets `Vary` (for example `Accept-Encoding`).
    #[must_use]
    pub fn with_vary(mut self, value: impl Into<String>) -> Self {
        self.vary = Some(value.into());
        self
    }

    /// Applies configured headers onto `response`.
    #[must_use]
    pub fn apply(self, mut response: Response) -> Response {
        if let Some(value) = self.cache_control {
            response = response.with_header("cache-control", value);
        }
        if let Some(value) = self.etag {
            response = response.with_header("etag", value);
        }
        if let Some(value) = self.last_modified {
            response = response.with_header("last-modified", value);
        }
        if let Some(value) = self.vary {
            response = response.with_header("vary", value);
        }
        response
    }
}

/// Strong `ETag`: `"tag"` (quotes added when missing).
#[must_use]
pub fn etag(tag: &str) -> String {
    quote_etag(tag, false)
}

/// Weak `ETag`: `W/"tag"`.
#[must_use]
pub fn weak_etag(tag: &str) -> String {
    quote_etag(tag, true)
}

fn quote_etag(tag: &str, weak: bool) -> String {
    let inner = tag.trim();
    let quoted = if inner.starts_with('"') && inner.ends_with('"') && inner.len() >= 2 {
        inner.to_owned()
    } else {
        format!("\"{inner}\"")
    };
    if weak { format!("W/{quoted}") } else { quoted }
}

/// Returns a **304 Not Modified** when `request` is a conditional GET/HEAD that
/// matches validators on `response`; otherwise returns `response` unchanged.
///
/// Precedence: `If-None-Match` (vs `ETag`) wins over `If-Modified-Since`
/// (vs `Last-Modified`) when both are present.
#[must_use]
pub fn maybe_not_modified(request: &Request, response: Response) -> Response {
    if !matches!(request.method(), Method::Get | Method::Head) {
        return response;
    }
    if !validators_match(request, &response) {
        return response;
    }
    not_modified_from(&response)
}

fn validators_match(request: &Request, response: &Response) -> bool {
    if let Some(inm) = request.headers().get("if-none-match") {
        let Some(etag) = response.headers().get("etag") else {
            return false;
        };
        return etag_matches(inm, etag);
    }
    if let Some(ims) = request.headers().get("if-modified-since") {
        let Some(last_modified) = response.headers().get("last-modified") else {
            return false;
        };
        return ims.trim().eq_ignore_ascii_case(last_modified.trim());
    }
    false
}

fn etag_matches(if_none_match: &str, response_etag: &str) -> bool {
    let response_norm = normalize_etag(response_etag);
    for part in if_none_match.split(',') {
        let candidate = part.trim();
        if candidate.is_empty() {
            continue;
        }
        if candidate == "*" {
            return true;
        }
        if normalize_etag(candidate) == response_norm {
            return true;
        }
    }
    false
}

fn normalize_etag(value: &str) -> String {
    let trimmed = value.trim();
    let without_weak = trimmed
        .strip_prefix("W/")
        .or_else(|| trimmed.strip_prefix("w/"))
        .unwrap_or(trimmed)
        .trim();
    without_weak.to_owned()
}

fn not_modified_from(response: &Response) -> Response {
    let mut out = Response::new(304);
    for name in ["cache-control", "etag", "last-modified", "vary", "expires"] {
        if let Some(value) = response.headers().get(name) {
            out = out.with_header(name, value.to_owned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        HttpCacheHeaders, etag, etag_matches, maybe_not_modified, normalize_etag, weak_etag,
    };
    use crate::{Method, Request, Response};

    #[test]
    fn builders_set_headers() {
        let response = HttpCacheHeaders::public_max_age(60)
            .with_etag(etag("v1"))
            .with_last_modified("Wed, 21 Oct 2015 07:28:00 GMT")
            .with_vary("Accept-Encoding")
            .apply(Response::text(200, "body"));
        assert_eq!(
            response.headers().get("cache-control"),
            Some("public, max-age=60")
        );
        assert_eq!(response.headers().get("etag"), Some("\"v1\""));
        assert_eq!(
            response.headers().get("last-modified"),
            Some("Wed, 21 Oct 2015 07:28:00 GMT")
        );
        assert_eq!(response.headers().get("vary"), Some("Accept-Encoding"));

        let private = HttpCacheHeaders::private_max_age(10).apply(Response::new(200));
        assert_eq!(
            private.headers().get("cache-control"),
            Some("private, max-age=10")
        );
        let no_store = HttpCacheHeaders::no_store().apply(Response::new(200));
        assert_eq!(no_store.headers().get("cache-control"), Some("no-store"));
    }

    #[test]
    fn etag_helpers_quote_and_weak() {
        assert_eq!(etag("abc"), "\"abc\"");
        assert_eq!(etag("\"abc\""), "\"abc\"");
        assert_eq!(weak_etag("abc"), "W/\"abc\"");
        assert_eq!(normalize_etag("W/\"x\""), "\"x\"");
        assert!(etag_matches("W/\"x\", \"y\"", "\"x\""));
        assert!(etag_matches("*", "\"anything\""));
        assert!(!etag_matches("\"a\"", "\"b\""));
    }

    #[test]
    fn maybe_not_modified_on_etag() {
        let response = HttpCacheHeaders::public_max_age(30)
            .with_etag(etag("1"))
            .apply(Response::text(200, "payload"));
        let request = Request::new(Method::Get, "/x").with_header("If-None-Match", "\"1\"");
        let not_mod = maybe_not_modified(&request, response.clone());
        assert_eq!(not_mod.status(), 304);
        assert_eq!(not_mod.body(), &[] as &[u8]);
        assert_eq!(not_mod.headers().get("etag"), Some("\"1\""));
        assert_eq!(
            not_mod.headers().get("cache-control"),
            Some("public, max-age=30")
        );

        let miss = Request::new(Method::Get, "/x").with_header("If-None-Match", "\"2\"");
        assert_eq!(maybe_not_modified(&miss, response).status(), 200);
    }

    #[test]
    fn maybe_not_modified_on_last_modified() {
        let lm = "Wed, 21 Oct 2015 07:28:00 GMT";
        let response = HttpCacheHeaders::new()
            .with_last_modified(lm)
            .apply(Response::text(200, "x"));
        let request = Request::new(Method::Head, "/").with_header("If-Modified-Since", lm);
        assert_eq!(maybe_not_modified(&request, response.clone()).status(), 304);

        let post = Request::new(Method::Post, "/").with_header("If-Modified-Since", lm);
        assert_eq!(maybe_not_modified(&post, response).status(), 200);
    }

    #[test]
    fn if_none_match_wins_over_if_modified_since() {
        let response = HttpCacheHeaders::new()
            .with_etag(etag("a"))
            .with_last_modified("Wed, 21 Oct 2015 07:28:00 GMT")
            .apply(Response::text(200, "x"));
        let request = Request::new(Method::Get, "/")
            .with_header("If-None-Match", "\"b\"")
            .with_header("If-Modified-Since", "Wed, 21 Oct 2015 07:28:00 GMT");
        // ETag mismatch → 200 even though Last-Modified matches.
        assert_eq!(maybe_not_modified(&request, response).status(), 200);
    }

    #[test]
    fn custom_cache_control() {
        let response = HttpCacheHeaders::new()
            .with_cache_control("max-age=0, must-revalidate")
            .apply(Response::new(204));
        assert_eq!(
            response.headers().get("cache-control"),
            Some("max-age=0, must-revalidate")
        );
    }

    #[test]
    fn conditional_misses_without_validators_or_headers() {
        let bare = Response::text(200, "x");
        let with_inm = Request::new(Method::Get, "/").with_header("If-None-Match", "\"a\"");
        assert_eq!(maybe_not_modified(&with_inm, bare.clone()).status(), 200);

        let with_ims = Request::new(Method::Get, "/")
            .with_header("If-Modified-Since", "Wed, 21 Oct 2015 07:28:00 GMT");
        assert_eq!(maybe_not_modified(&with_ims, bare.clone()).status(), 200);

        let plain = Request::new(Method::Get, "/");
        assert_eq!(maybe_not_modified(&plain, bare).status(), 200);

        assert!(etag_matches(" , \"a\" , ", "\"a\""));
    }
}
