//! HTML toolbar injection.

use serenade_http::Response;

/// Injects a compact debug toolbar before `</body>` when the response is HTML.
#[must_use]
pub fn inject_toolbar(response: Response, token: &str, path_prefix: &str) -> Response {
    let Some(content_type) = response.headers().get("content-type") else {
        return response;
    };
    if !content_type.to_ascii_lowercase().contains("text/html") {
        return response;
    }
    let Some(body) = response.body_str() else {
        return response;
    };
    let marker = "</body>";
    let Some(idx) = body.to_ascii_lowercase().rfind(marker) else {
        return response;
    };
    let link = format!("{path_prefix}/{token}");
    let bar = format!(
        r#"<div id="sf-toolbar" style="position:fixed;bottom:0;left:0;right:0;z-index:99999;background:#222;color:#eee;font:12px/1.4 ui-monospace,monospace;padding:6px 10px;border-top:2px solid #0a0;">Serenade profiler <a href="{link}" style="color:#7f7;">#{token}</a> · status {status}</div>"#,
        link = html_escape(&link),
        token = html_escape(token),
        status = response.status(),
    );
    let mut new_body = String::with_capacity(body.len() + bar.len());
    new_body.push_str(&body[..idx]);
    new_body.push_str(&bar);
    new_body.push_str(&body[idx..]);
    response.with_body(new_body.into_bytes())
}

fn html_escape(input: &str) -> String {
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

#[cfg(test)]
mod toolbar_tests {
    use super::*;
    use serenade_http::Response;

    #[test]
    fn injects_before_body_close() {
        let response = Response::new(200)
            .with_header("content-type", "text/html; charset=utf-8")
            .with_body(b"<html><body>hi</body></html>".to_vec());
        let out = inject_toolbar(response, "abc", "/_profiler");
        let text = out.body_str().expect("utf8");
        assert!(text.contains("sf-toolbar"));
        assert!(text.contains("/_profiler/abc"));
        assert!(text.contains("</body>"));
    }

    #[test]
    fn skips_non_html() {
        let response = Response::text(200, "plain");
        let out = inject_toolbar(response, "abc", "/_profiler");
        assert_eq!(out.body_str(), Some("plain"));
    }
}
