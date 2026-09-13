//! Unit and feature tests for `serenade-http-client`.

use std::time::Duration;

use std::sync::Arc;

use serenade_di::{CompilePass, ContainerBuilder, ServiceDefinition};

use crate::{
    ClientMethod, ClientRequest, ClientResponse, DEFAULT_HTTP_CLIENT_SERVICE, DynHttpClient,
    HTTP_CLIENT_TAG, HttpClient, HttpClientError, HttpClientService, MockHttpClient,
    RegisterDefaultHttpClientPass, version,
};

#[test]
fn version_is_nonempty() {
    assert_ne!(version(), "");
}

#[test]
fn client_request_builders() {
    let request = ClientRequest::post("https://example.test/api")
        .header("Authorization", "Bearer x")
        .body_text("hi")
        .timeout(Duration::from_secs(2));
    assert_eq!(request.method(), ClientMethod::Post);
    assert_eq!(request.url(), "https://example.test/api");
    assert_eq!(
        request.headers().get("authorization").map(String::as_str),
        Some("Bearer x")
    );
    assert_eq!(request.body_bytes(), b"hi");
    assert_eq!(request.timeout_duration(), Some(Duration::from_secs(2)));
    assert_eq!(
        request.headers().get("content-type").map(String::as_str),
        Some("text/plain; charset=utf-8")
    );

    assert_eq!(ClientRequest::get("u").method(), ClientMethod::Get);
    assert_eq!(ClientRequest::put("u").method(), ClientMethod::Put);
    assert_eq!(ClientRequest::patch("u").method(), ClientMethod::Patch);
    assert_eq!(ClientRequest::delete("u").method(), ClientMethod::Delete);
    assert_eq!(ClientRequest::head("u").method(), ClientMethod::Head);
    assert_eq!(ClientMethod::Get.as_str(), "GET");
    assert_eq!(ClientMethod::Post.as_str(), "POST");
    assert_eq!(ClientMethod::Put.as_str(), "PUT");
    assert_eq!(ClientMethod::Patch.to_string(), "PATCH");
    assert_eq!(ClientMethod::Head.as_str(), "HEAD");
    assert_eq!(ClientMethod::Delete.as_str(), "DELETE");
    assert_eq!(ClientMethod::Get.to_string(), "GET");

    let keep_type = ClientRequest::post("u")
        .header("content-type", "application/json")
        .body_text("{}");
    assert_eq!(
        keep_type.headers().get("content-type").map(String::as_str),
        Some("application/json")
    );
}

#[test]
fn client_response_accessors() {
    let response = ClientResponse::new(404)
        .header("X-Trace", "abc")
        .body(vec![0xff, 0xfe])
        .body_text("nope");
    assert_eq!(response.status(), 404);
    assert!(!response.is_success());
    assert_eq!(response.header_value("x-trace"), Some("abc"));
    assert!(response.headers().contains_key("x-trace"));
    assert_eq!(response.body_bytes(), b"nope");
    assert_eq!(response.body_text_lossy(), "nope");
    assert!(ClientResponse::new(204).is_success());
}

#[tokio::test]
async fn mock_unlimited_expectation() {
    let mock = MockHttpClient::new();
    mock.expect(
        ClientMethod::Get,
        "https://example.test/loop",
        ClientResponse::new(200),
        None,
    );
    assert!(
        mock.get("https://example.test/loop")
            .await
            .expect("1")
            .is_success()
    );
    assert!(
        mock.get("https://example.test/loop")
            .await
            .expect("2")
            .is_success()
    );
}

#[tokio::test]
async fn mock_get_and_post_helpers() {
    let mock = MockHttpClient::new();
    mock.expect(
        ClientMethod::Get,
        "https://example.test/a",
        ClientResponse::new(200).body_text("ok"),
        Some(1),
    );
    mock.expect(
        ClientMethod::Post,
        "https://example.test/b",
        ClientResponse::new(201).body_text("created"),
        Some(1),
    );

    let get = mock.get("https://example.test/a").await.expect("get");
    assert!(get.is_success());
    assert_eq!(get.body_text_lossy(), "ok");

    let post = mock
        .post("https://example.test/b", b"{}".to_vec())
        .await
        .expect("post");
    assert_eq!(post.status(), 201);

    let miss = mock.get("https://example.test/missing").await;
    assert!(matches!(
        miss,
        Err(HttpClientError::MockMiss { method, url })
            if method == "GET" && url == "https://example.test/missing"
    ));
}

#[tokio::test]
async fn mock_expect_where_and_times() {
    let mock = MockHttpClient::new();
    mock.expect_where(
        |request| request.url().contains("/filtered"),
        ClientResponse::new(204),
        Some(2),
    );
    assert_eq!(
        mock.send(ClientRequest::get("https://example.test/filtered/1"))
            .await
            .expect("1")
            .status(),
        204
    );
    assert_eq!(
        mock.send(ClientRequest::delete("https://example.test/filtered/2"))
            .await
            .expect("2")
            .status(),
        204
    );
    let err = mock
        .send(ClientRequest::get("https://example.test/filtered/3"))
        .await;
    assert!(matches!(err, Err(HttpClientError::MockMiss { .. })));
}

#[cfg(feature = "reqwest")]
mod reqwest_tests {
    use super::*;
    use crate::ReqwestHttpClient;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn reqwest_client_get_against_wiremock() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/hello"))
            .respond_with(ResponseTemplate::new(200).set_body_string("world"))
            .mount(&server)
            .await;

        let client = ReqwestHttpClient::with_timeout(Duration::from_secs(5));
        assert_eq!(client.default_timeout(), Duration::from_secs(5));
        let url = format!("{}/hello", server.uri());
        let response = client.get(url).await.expect("send");
        assert_eq!(response.status(), 200);
        assert_eq!(response.body_text_lossy(), "world");
    }

    #[tokio::test]
    async fn reqwest_client_post_json_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/echo"))
            .respond_with(ResponseTemplate::new(202).set_body_string("accepted"))
            .mount(&server)
            .await;

        let client = ReqwestHttpClient::new();
        let response = client
            .send(
                ClientRequest::post(format!("{}/echo", server.uri()))
                    .header("content-type", "application/json")
                    .body(br#"{"a":1}"#.to_vec())
                    .timeout(Duration::from_secs(3)),
            )
            .await
            .expect("post");
        assert_eq!(response.status(), 202);
        assert_eq!(response.body_text_lossy(), "accepted");
    }

    #[tokio::test]
    async fn reqwest_client_maps_other_methods() {
        let server = MockServer::start().await;
        for (verb, path_suffix) in [
            ("PUT", "/put"),
            ("PATCH", "/patch"),
            ("DELETE", "/delete"),
            ("HEAD", "/head"),
        ] {
            Mock::given(method(verb))
                .and(path(path_suffix))
                .respond_with(ResponseTemplate::new(200).set_body_string(verb))
                .mount(&server)
                .await;
        }

        let client = ReqwestHttpClient::new();
        let base = server.uri();
        assert_eq!(
            client
                .send(ClientRequest::put(format!("{base}/put")))
                .await
                .expect("put")
                .body_text_lossy(),
            "PUT"
        );
        assert_eq!(
            client
                .send(ClientRequest::patch(format!("{base}/patch")))
                .await
                .expect("patch")
                .body_text_lossy(),
            "PATCH"
        );
        assert_eq!(
            client
                .send(ClientRequest::delete(format!("{base}/delete")))
                .await
                .expect("delete")
                .body_text_lossy(),
            "DELETE"
        );
        let head = client
            .send(ClientRequest::head(format!("{base}/head")))
            .await
            .expect("head");
        assert_eq!(head.status(), 200);
    }

    #[tokio::test]
    async fn reqwest_client_transport_error() {
        let client = ReqwestHttpClient::with_timeout(Duration::from_millis(50));
        let err = client
            .get("http://127.0.0.1:1/")
            .await
            .expect_err("should fail");
        assert!(matches!(err, HttpClientError::Transport { .. }));
    }
}

#[test]
fn http_client_error_display() {
    assert!(
        HttpClientError::Request {
            message: "bad".to_owned()
        }
        .to_string()
        .contains("request")
    );
    assert!(
        HttpClientError::Transport {
            message: "down".to_owned()
        }
        .to_string()
        .contains("transport")
    );
}

#[tokio::test]
async fn http_client_service_wraps_mock() {
    let mock = MockHttpClient::new();
    mock.expect(
        ClientMethod::Get,
        "https://example.test/svc",
        ClientResponse::new(200).body_text("svc"),
        Some(1),
    );
    let service = HttpClientService(Arc::new(mock) as Arc<dyn DynHttpClient>);
    let response = service.get("https://example.test/svc").await.expect("get");
    assert_eq!(response.body_text_lossy(), "svc");
}

#[test]
fn compile_pass_name() {
    assert_eq!(
        RegisterDefaultHttpClientPass.name(),
        "register_default_http_client"
    );
}

#[cfg(feature = "reqwest")]
#[test]
fn compile_pass_registers_reqwest_client() {
    let mut builder = ContainerBuilder::new();
    builder.add_compile_pass(RegisterDefaultHttpClientPass);
    let container = builder.compile().expect("compile");
    let _client = container
        .get_as::<HttpClientService>(DEFAULT_HTTP_CLIENT_SERVICE)
        .expect("http_client");
}

#[test]
fn compile_pass_skips_when_default_already_registered() {
    let mut builder = ContainerBuilder::new();
    builder
        .register(
            ServiceDefinition::new(DEFAULT_HTTP_CLIENT_SERVICE).with_tag(HTTP_CLIENT_TAG),
            |_| {
                Ok(Box::new(HttpClientService(
                    Arc::new(MockHttpClient::new()) as Arc<dyn DynHttpClient>
                )))
            },
        )
        .expect("register");
    builder.add_compile_pass(RegisterDefaultHttpClientPass);
    let container = builder.compile().expect("compile");
    assert!(
        container
            .get_as::<HttpClientService>(DEFAULT_HTTP_CLIENT_SERVICE)
            .is_ok()
    );
}
