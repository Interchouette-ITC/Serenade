//! Unit tests for `serenade-profiler`.

use std::sync::Arc;
use std::time::Duration;

use serenade_http::{
    AsyncHttpKernel, HttpError, HttpKernel, Method, ROUTE_ATTRIBUTE, Request, Response,
};
use tracing_subscriber::prelude::*;

use crate::{
    AsyncProfilerMiddleware, FORCE_TOKEN_FALLBACK, PROFILER_TOKEN_ATTRIBUTE, ProfileStore,
    ProfilerConfig, ProfilerLogLayer, ProfilerMiddleware, QueryEvent, generate_token,
    inject_toolbar, profiler_detail_html, profiler_index_html, record_query, try_handle_profiler,
    version, with_profile_scope,
};

#[test]
fn crate_version_and_token_paths() {
    assert_ne!(generate_token(), "");
    assert_ne!(version(), "");
    FORCE_TOKEN_FALLBACK.with(|c| c.set(true));
    let fallback = generate_token();
    FORCE_TOKEN_FALLBACK.with(|c| c.set(false));
    assert_eq!(fallback.len(), 16);
}

#[test]
fn config_defaults_and_builders() {
    let disabled = ProfilerConfig::disabled();
    assert!(!disabled.enabled);
    let enabled = ProfilerConfig::enabled(0).with_path_prefix("/dbg");
    assert!(enabled.enabled);
    assert_eq!(enabled.capacity, 1);
    assert_eq!(enabled.path_prefix, "/dbg");
}

#[test]
fn store_evicts_oldest_and_records_query() {
    let store = Arc::new(ProfileStore::new(2));
    assert!(store.is_empty());
    for i in 0..3 {
        let mut data = crate::ProfileData::new(format!("t{i}"), "GET".into(), "/".into());
        data.status = 200;
        store.insert(data);
    }
    assert_eq!(store.len(), 2);
    assert!(store.get("t0").is_none());
    assert!(store.get("t1").is_some());
    record_query(
        &store,
        "t2",
        QueryEvent::new("select 1", Duration::from_millis(3)).with_binds("[]"),
    );
    let profile = store.get("t2").expect("t2");
    assert_eq!(profile.queries.len(), 1);
    assert_eq!(profile.queries[0].sql, "select 1");
}

#[test]
fn store_ensure_updates_and_evicts() {
    let store = Arc::new(ProfileStore::new(1));
    store.ensure(crate::ProfileData::new(
        "a".into(),
        "GET".into(),
        "/a".into(),
    ));
    store.ensure(crate::ProfileData::new(
        "a".into(),
        "POST".into(),
        "/a2".into(),
    ));
    let updated = store.get("a").expect("a");
    assert_eq!(updated.method, "POST");
    assert_eq!(updated.path, "/a2");
    store.ensure(crate::ProfileData::new(
        "b".into(),
        "GET".into(),
        "/b".into(),
    ));
    assert!(store.get("a").is_none());
    assert!(store.get("b").is_some());
    store.push_query("missing", QueryEvent::new("x", Duration::ZERO));
    store.push_log(
        "missing",
        crate::LogLine {
            target: "t".into(),
            level: "INFO".into(),
            message: "m".into(),
        },
    );
    store.set_route("missing", "r".into());
    store.finish("missing", 500, Duration::from_millis(1));
}

#[test]
fn sync_middleware_injects_toolbar_and_skips_when_disabled() {
    let store = Arc::new(ProfileStore::new(10));
    let mut kernel = HttpKernel::new(|request: &mut Request| {
        assert!(
            request
                .attributes()
                .get::<String>(PROFILER_TOKEN_ATTRIBUTE)
                .is_some()
        );
        request
            .attributes_mut()
            .insert(ROUTE_ATTRIBUTE, "feed".to_owned());
        Ok(Response::new(200)
            .with_header("content-type", "text/html")
            .with_body(b"<html><body>ok</body></html>".to_vec()))
    });
    kernel.push_middleware(ProfilerMiddleware::new(
        Arc::clone(&store),
        ProfilerConfig::enabled(10),
    ));
    let response = kernel.handle(Request::new(Method::Get, "/feed"));
    assert!(response.body_str().unwrap().contains("sf-toolbar"));
    assert_eq!(store.len(), 1);
    assert_eq!(store.list()[0].route.as_deref(), Some("feed"));

    let mut off = HttpKernel::new(|_: &mut Request| Ok(Response::text(200, "x")));
    off.push_middleware(ProfilerMiddleware::new(
        Arc::clone(&store),
        ProfilerConfig::disabled(),
    ));
    let plain = off.handle(Request::new(Method::Get, "/"));
    assert_eq!(plain.body_str(), Some("x"));
}

#[test]
fn sync_middleware_records_errors() {
    let store = Arc::new(ProfileStore::new(5));
    let mut kernel = HttpKernel::new(|_: &mut Request| Err(HttpError::status(404, "missing")));
    kernel.push_middleware(ProfilerMiddleware::new(
        Arc::clone(&store),
        ProfilerConfig::enabled(5),
    ));
    let response = kernel.handle(Request::new(Method::Get, "/gone"));
    assert_eq!(response.status(), 404);
    let profile = store.list().into_iter().next().expect("profile");
    assert_eq!(profile.status, 404);
}

#[tokio::test]
async fn async_middleware_profiles_html() {
    let store = Arc::new(ProfileStore::new(5));
    let mut kernel = AsyncHttpKernel::from_sync(|request: &mut Request| {
        request
            .attributes_mut()
            .insert(ROUTE_ATTRIBUTE, "home".to_owned());
        Ok(Response::new(200)
            .with_header("content-type", "text/html; charset=utf-8")
            .with_body(b"<html><body>hi</body></html>".to_vec()))
    });
    kernel.push_middleware(AsyncProfilerMiddleware::new(
        Arc::clone(&store),
        ProfilerConfig::enabled(5),
    ));
    let response = kernel.handle(Request::new(Method::Get, "/")).await;
    assert!(response.body_str().unwrap().contains("sf-toolbar"));
    assert!(!store.is_empty());
}

#[tokio::test]
async fn async_middleware_disabled_and_errors() {
    let store = Arc::new(ProfileStore::new(5));
    let mut off = AsyncHttpKernel::from_sync(|_: &mut Request| Ok(Response::text(200, "plain")));
    off.push_middleware(AsyncProfilerMiddleware::new(
        Arc::clone(&store),
        ProfilerConfig::disabled(),
    ));
    let plain = off.handle(Request::new(Method::Get, "/")).await;
    assert_eq!(plain.body_str(), Some("plain"));
    assert!(store.is_empty());

    let mut err = AsyncHttpKernel::from_sync(|_: &mut Request| Err(HttpError::status(500, "boom")));
    err.push_middleware(AsyncProfilerMiddleware::new(
        Arc::clone(&store),
        ProfilerConfig::enabled(5),
    ));
    let response = err.handle(Request::new(Method::Get, "/boom")).await;
    assert_eq!(response.status(), 500);
    assert_eq!(store.list()[0].status, 500);
}

#[test]
fn try_handle_covers_index_detail_and_404() {
    let store = Arc::new(ProfileStore::new(5));
    let mut data = crate::ProfileData::new("deadbeef".into(), "GET".into(), "/".into());
    data.status = 200;
    data.route = Some("home".into());
    data.logs.push(crate::LogLine {
        target: "serenade::request".into(),
        level: "INFO".into(),
        message: "hello".into(),
    });
    data.queries
        .push(QueryEvent::new("select 1", Duration::from_millis(1)).with_binds("[]"));
    store.insert(data);

    let index = try_handle_profiler(
        &store,
        "/_profiler",
        &Request::new(Method::Get, "/_profiler"),
    )
    .expect("index");
    assert!(index.body_str().unwrap().contains("deadbeef"));

    let trailing = try_handle_profiler(
        &store,
        "/_profiler",
        &Request::new(Method::Get, "/_profiler/"),
    )
    .expect("trailing slash index");
    assert_eq!(trailing.status(), 200);

    let detail = try_handle_profiler(
        &store,
        "/_profiler",
        &Request::new(Method::Get, "/_profiler/deadbeef"),
    )
    .expect("detail");
    assert!(detail.body_str().unwrap().contains("select 1"));
    assert!(detail.body_str().unwrap().contains("hello"));
    assert!(detail.body_str().unwrap().contains("binds"));

    let missing = try_handle_profiler(
        &store,
        "/_profiler",
        &Request::new(Method::Get, "/_profiler/missing"),
    )
    .expect("404");
    assert_eq!(missing.status(), 404);

    assert!(
        try_handle_profiler(
            &store,
            "/_profiler",
            &Request::new(Method::Post, "/_profiler")
        )
        .is_none()
    );
    assert!(
        try_handle_profiler(&store, "/_profiler", &Request::new(Method::Get, "/other")).is_none()
    );
    assert!(
        try_handle_profiler(
            &store,
            "/_profiler",
            &Request::new(Method::Get, "/_profiler/a/b")
        )
        .is_none()
    );
}

#[test]
fn ui_empty_panels_and_escape() {
    let store = Arc::new(ProfileStore::new(2));
    let empty = profiler_index_html(&store, "/_profiler");
    assert!(empty.contains("No profiles yet."));

    let mut data = crate::ProfileData::new("t<>&\"'".into(), "GET".into(), "/p".into());
    data.status = 200;
    let detail = profiler_detail_html(&data, "/_profiler");
    assert!(detail.contains("No queries recorded."));
    assert!(detail.contains("No log lines captured."));
    assert!(detail.contains("&lt;"));
    assert!(detail.contains("&amp;"));
    assert!(detail.contains("&quot;"));
    assert!(detail.contains("&#39;"));
}

#[test]
fn log_layer_captures_scoped_events() {
    let store = Arc::new(ProfileStore::new(5));
    store.ensure(crate::ProfileData::new(
        "tok".into(),
        "GET".into(),
        "/".into(),
    ));
    let _subscriber = tracing_subscriber::registry()
        .with(ProfilerLogLayer)
        .set_default();
    with_profile_scope(&store, "tok".into(), || {
        tracing::info!(target: "serenade::request", "scoped line");
    });
    let profile = store.get("tok").expect("tok");
    assert!(
        profile
            .logs
            .iter()
            .any(|line| line.message.contains("scoped line"))
    );
}

#[test]
fn inject_toolbar_edge_cases() {
    let no_ct = inject_toolbar(
        Response::new(200).with_body(b"<html><body></body></html>".to_vec()),
        "x",
        "/_profiler",
    );
    assert!(!no_ct.body_str().unwrap().contains("sf-toolbar"));

    let bad_utf8 = inject_toolbar(
        Response::new(200)
            .with_header("content-type", "text/html")
            .with_body(vec![0xff, 0xfe]),
        "x",
        "/_profiler",
    );
    assert_eq!(bad_utf8.body(), &[0xff, 0xfe]);

    let response = Response::new(200)
        .with_header("content-type", "text/html")
        .with_body(b"<html>no close".to_vec());
    let out = inject_toolbar(response, "x", "/_profiler");
    assert_eq!(out.body_str(), Some("<html>no close"));

    let escaped = inject_toolbar(
        Response::new(200)
            .with_header("content-type", "text/html")
            .with_body(b"<html><body>ok</body></html>".to_vec()),
        "a&b<'\">",
        "/_profiler",
    );
    let text = escaped.body_str().unwrap();
    assert!(text.contains("&amp;"));
    assert!(text.contains("&lt;"));
    assert!(text.contains("&gt;"));
    assert!(text.contains("&quot;"));
    assert!(text.contains("&#39;"));
}
