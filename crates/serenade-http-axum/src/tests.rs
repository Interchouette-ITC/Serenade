use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Method as HttpMethod, Request as HttpRequest};
use http_body_util::BodyExt;
use serenade_http::{AsyncHttpKernel, HttpKernel, Method, Request, Response};
use tower::ServiceExt;

use super::{
    await_bound, bind_server, conversion_error, dispatch, dispatch_async, from_axum, listen,
    router, to_axum, version,
};

#[test]
fn version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn to_axum_preserves_status_and_body() {
    let response = to_axum(&Response::text(201, "created"));
    assert_eq!(response.status(), 201);
}

#[tokio::test]
async fn router_default_fallback_dispatches_kernel() {
    let kernel = AsyncHttpKernel::from_sync(|request: &mut Request| {
        assert_eq!(request.path(), "/any");
        Ok(Response::text(200, "served"))
    });
    let app = router(Arc::new(kernel));
    let response = app
        .oneshot(
            HttpRequest::builder()
                .uri("/any")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("oneshot");
    assert!(response.status().is_success());
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    assert_eq!(body.as_ref(), b"served");
}

#[tokio::test]
async fn async_dispatch_awaits_controller() {
    use serenade_http::box_future;

    let kernel = AsyncHttpKernel::from_async_fn(|request: &mut Request| {
        let path = request.path().to_owned();
        box_future(async move { Ok(Response::text(200, format!("async:{path}"))) })
    });
    let app = router(Arc::new(kernel));
    let response = app
        .oneshot(
            HttpRequest::builder()
                .uri("/async")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("oneshot");
    assert!(response.status().is_success());
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    assert_eq!(body.as_ref(), b"async:/async");
}

#[tokio::test]
async fn from_axum_copies_headers_body_and_query() {
    let request = HttpRequest::builder()
        .method(HttpMethod::POST)
        .uri("/echo?_locale=fr&x=1")
        .header("x-trace", "abc")
        .body(())
        .expect("request");
    let (parts, ()) = request.into_parts();
    let serenade = from_axum(&parts, b"ping").expect("convert");
    assert_eq!(serenade.path(), "/echo");
    assert_eq!(serenade.query(), Some("_locale=fr&x=1"));
    assert_eq!(serenade.headers().get("x-trace"), Some("abc"));
    assert_eq!(serenade.body(), b"ping");
}

#[tokio::test]
async fn from_axum_rejects_unsupported_method() {
    let request = HttpRequest::builder()
        .method(HttpMethod::TRACE)
        .uri("/")
        .body(())
        .expect("request");
    let (parts, ()) = request.into_parts();
    let error = from_axum(&parts, []).expect_err("TRACE unsupported");
    assert_eq!(error.status_code(), 405);
}

#[test]
fn conversion_error_maps_to_405_response() {
    let error = serenade_http::HttpError::status(405, "method not allowed");
    let response = conversion_error(&error);
    assert_eq!(response.status(), 405);
}

#[test]
fn dispatch_maps_from_axum_errors() {
    let kernel = HttpKernel::new(|_: &mut Request| Ok(Response::text(200, "ok")));
    let request = HttpRequest::builder()
        .method(HttpMethod::TRACE)
        .uri("/")
        .body(())
        .expect("request");
    let (parts, ()) = request.into_parts();
    let response = dispatch(&kernel, &parts, []);
    assert_eq!(response.status(), 405);
}

#[tokio::test]
async fn dispatch_async_maps_from_axum_errors() {
    let kernel = AsyncHttpKernel::from_sync(|_: &mut Request| Ok(Response::text(200, "ok")));
    let request = HttpRequest::builder()
        .method(HttpMethod::TRACE)
        .uri("/")
        .body(())
        .expect("request");
    let (parts, ()) = request.into_parts();
    let response = dispatch_async(&kernel, &parts, []).await;
    assert_eq!(response.status(), 405);
}

#[tokio::test]
async fn sync_dispatch_via_router_state() {
    let kernel = AsyncHttpKernel::from_sync(|request: &mut Request| {
        assert_eq!(request.method(), Method::Get);
        assert_eq!(request.path(), "/healthz");
        Ok(Response::text(200, "ok"))
    });
    let app = router(Arc::new(kernel));
    let response = app
        .oneshot(
            HttpRequest::builder()
                .uri("/healthz")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("oneshot");
    assert!(response.status().is_success());
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    assert_eq!(body.as_ref(), b"ok");
}

#[tokio::test]
async fn listen_binds_serves_then_stops() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let kernel =
        AsyncHttpKernel::from_sync(|_request: &mut Request| Ok(Response::text(200, "listen-ok")));
    let (server, handle) = bind_server("127.0.0.1:0", kernel).await.expect("bind");
    let addr = server.local_addr();
    let task = tokio::spawn(await_bound(server));

    let mut body = None;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(20)).await;
        let Ok(mut stream) = tokio::net::TcpStream::connect(addr).await else {
            continue;
        };
        let request = format!("GET / HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        if stream.write_all(request.as_bytes()).await.is_err() {
            continue;
        }
        let mut buf = Vec::new();
        if stream.read_to_end(&mut buf).await.is_err() || buf.is_empty() {
            continue;
        }
        body = Some(buf);
        break;
    }

    handle.shutdown();
    task.await.expect("join").expect("server stopped cleanly");
    let body = body.expect("listen did not become ready");
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("200"), "{text}");
    assert!(text.contains("listen-ok"), "{text}");
}

#[tokio::test]
async fn listen_awaits_until_server_stops() {
    let kernel = AsyncHttpKernel::from_sync(|_request: &mut Request| Ok(Response::text(200, "ok")));
    let (server, handle) = bind_server("127.0.0.1:0", kernel).await.expect("bind");
    let task = tokio::spawn(await_bound(server));
    tokio::time::sleep(Duration::from_millis(40)).await;
    handle.shutdown();
    task.await.expect("join").expect("listen await completed");
}

#[tokio::test]
async fn listen_public_entry_binds_and_serves() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let kernel =
        AsyncHttpKernel::from_sync(|_request: &mut Request| Ok(Response::text(200, "entry-ok")));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("probe");
    let addr = listener.local_addr().expect("addr");
    drop(listener);

    let task = tokio::spawn(async move { listen(addr, kernel).await });

    let mut body = None;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(20)).await;
        let Ok(mut stream) = tokio::net::TcpStream::connect(addr).await else {
            continue;
        };
        let request = format!("GET / HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        if stream.write_all(request.as_bytes()).await.is_err() {
            continue;
        }
        let mut buf = Vec::new();
        if stream.read_to_end(&mut buf).await.is_err() || buf.is_empty() {
            continue;
        }
        body = Some(buf);
        break;
    }

    assert!(
        body.as_ref()
            .is_some_and(|b| String::from_utf8_lossy(b).contains("entry-ok")),
        "listen entry did not become ready"
    );
    task.abort();
    let _ = task.await;
}

#[tokio::test]
async fn listen_propagates_bind_error() {
    let kernel =
        AsyncHttpKernel::from_sync(|_request: &mut Request| Ok(Response::text(200, "unused")));
    // Privileged port fails for non-root runners (CI and local).
    let err = listen("127.0.0.1:1", kernel).await;
    assert!(err.is_err(), "expected bind failure on privileged port");
}
