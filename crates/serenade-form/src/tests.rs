use std::sync::Arc;

use serenade_http::{Method, Request};
use serenade_security::HmacCsrfTokenManager;
use serenade_validator::NotBlank;

use super::{Form, FormError, FormStatus, escape_attr, escape_html, parse_urlencoded, version};

#[test]
fn version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn escape_html_xss_payload() {
    let raw = r#"<script>alert("x")</script>&'"#;
    let escaped = escape_html(raw);
    assert!(!escaped.contains('<'));
    assert!(escaped.contains("&lt;script&gt;"));
    assert!(escaped.contains("&amp;"));
    assert!(escaped.contains("&quot;"));
    assert!(escaped.contains("&#39;"));
    assert_eq!(escape_attr(r#""quoted""#), "&quot;quoted&quot;");
}

#[test]
fn parse_urlencoded_basic() {
    let map = parse_urlencoded(b"a=hello+world&b=%3Ctag%3E").expect("parse");
    assert_eq!(map.get("a").map(String::as_str), Some("hello world"));
    assert_eq!(map.get("b").map(String::as_str), Some("<tag>"));
}

#[test]
fn parse_urlencoded_edge_cases() {
    assert!(parse_urlencoded(b"").expect("empty").is_empty());
    let map = parse_urlencoded(b"solo&x=1&=&%41=%42&&z=9").expect("pairs");
    assert_eq!(map.get("solo").map(String::as_str), Some(""));
    assert_eq!(map.get("x").map(String::as_str), Some("1"));
    assert_eq!(map.get("A").map(String::as_str), Some("B"));
    assert_eq!(map.get("z").map(String::as_str), Some("9"));
    assert!(matches!(
        parse_urlencoded(b"\xff"),
        Err(FormError::InvalidEncoding)
    ));
    assert!(matches!(
        parse_urlencoded(b"a=%zz"),
        Err(FormError::InvalidEncoding)
    ));
    assert!(matches!(
        parse_urlencoded(b"a=%a"),
        Err(FormError::InvalidEncoding)
    ));
}

#[test]
fn form_csrf_and_validate_roundtrip() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("comment")
        .method(Method::Post)
        .action("/comment")
        .field(
            "body",
            vec![Arc::new(NotBlank) as Arc<dyn serenade_validator::Constraint>],
        )
        .build();
    assert_eq!(form.name(), "comment");
    assert!(!form.is_submitted());
    form.prepare_csrf(&mgr).expect("prepare");
    let rendered = form.render().expect("render");
    let html = rendered.as_html();
    assert!(html.contains(r#"name="_token""#));
    assert!(html.contains("method=\"POST\""));
    assert!(html.contains(r#"action="/comment""#));
    assert_eq!(rendered.as_ref(), html);

    let token_value = html
        .split("value=\"")
        .nth(1)
        .expect("value")
        .split('"')
        .next()
        .expect("end")
        .to_owned();

    let body = format!("_token={token_value}&body=hello");
    let request = Request::new(Method::Post, "/").with_body(body.into_bytes());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert!(form.is_submitted());
    assert!(form.is_valid());
    assert!(form.violations().is_empty());
    assert_eq!(form.get("body"), Some("hello"));
    assert!(form.set("body", "updated"));
    assert_eq!(form.get("body"), Some("updated"));
    assert!(!form.set("missing", "x"));
    assert_eq!(form.get("missing"), None);
    assert_eq!(form.data().get("body").map(String::as_str), Some("updated"));
}

#[test]
fn form_validation_fails_blank() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("comment")
        .csrf(false)
        .field(
            "body",
            vec![Arc::new(NotBlank) as Arc<dyn serenade_validator::Constraint>],
        )
        .build();
    form.prepare_csrf(&mgr).expect("noop csrf");
    let request = Request::new(Method::Post, "/").with_body(b"body=".to_vec());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert!(!form.is_valid());
    assert!(!form.violations().is_empty());
}

#[test]
fn form_clears_missing_fields_and_put() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("comment")
        .csrf(false)
        .field("body", vec![])
        .build();
    let request = Request::new(Method::Put, "/").with_body(b"other=1".to_vec());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert_eq!(form.get("body"), Some(""));
}

#[test]
fn form_render_requires_csrf_token() {
    let form = Form::builder("comment").field("body", vec![]).build();
    let err = form.render().expect_err("no token");
    assert!(matches!(err, FormError::Csrf(_)));
}

#[test]
fn form_render_without_csrf() {
    let form = Form::builder("comment")
        .csrf(false)
        .field("body", vec![])
        .build();
    let rendered = form.render().expect("render");
    let html = rendered.as_html();
    assert!(!html.contains("_token"));
    assert!(html.contains(r#"name="body""#));
}

#[test]
fn form_rejects_bad_csrf() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("comment").field("body", vec![]).build();
    let request = Request::new(Method::Post, "/").with_body(b"_token=nope&body=x".to_vec());
    let err = form.handle_request(&request, &mgr).expect_err("csrf");
    assert!(matches!(err, FormError::Csrf(_)));
}

#[test]
fn form_get_is_not_submitted() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("comment").build();
    let request = Request::new(Method::Get, "/");
    assert_eq!(
        form.handle_request(&request, &mgr).expect("get"),
        FormStatus::NotSubmitted
    );
}
