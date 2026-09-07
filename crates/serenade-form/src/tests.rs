use std::sync::Arc;

use serenade_http::{Method, Request};
use serenade_security::HmacCsrfTokenManager;
use serenade_validator::NotBlank;

use super::{Form, FormError, FormStatus, escape_html, parse_urlencoded, version};

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
}

#[test]
fn parse_urlencoded_basic() {
    let map = parse_urlencoded(b"a=hello+world&b=%3Ctag%3E").expect("parse");
    assert_eq!(map.get("a").map(String::as_str), Some("hello world"));
    assert_eq!(map.get("b").map(String::as_str), Some("<tag>"));
}

#[test]
fn form_csrf_and_validate_roundtrip() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("comment")
        .field(
            "body",
            vec![Arc::new(NotBlank) as Arc<dyn serenade_validator::Constraint>],
        )
        .build();
    form.prepare_csrf(&mgr).expect("prepare");
    let html = form.render().expect("render").as_html().to_owned();
    assert!(html.contains(r#"name="_token""#));
    assert!(html.contains("method=\"POST\""));

    let token_value = form
        .render()
        .expect("render")
        .as_html()
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
    assert!(form.is_valid());
    assert_eq!(form.get("body"), Some("hello"));
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
