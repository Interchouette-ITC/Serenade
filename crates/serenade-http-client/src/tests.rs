//! Unit and feature tests for `serenade-http-client`.

use std::time::Duration;

use crate::{
    ClientMethod, ClientRequest, ClientResponse, HttpClient, HttpClientError, MockHttpClient,
    version,
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

        let client = ReqwestHttpClient::with_timeout(Duration::from_secs(5)).expect("client");
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

        let client = ReqwestHttpClient::new().expect("client");
        let response = client
            .send(
                ClientRequest::post(format!("{}/echo", server.uri()))
                    .header("content-type", "application/json")
                    .body(br#"{"a":1}"#.to_vec()),
            )
            .await
            .expect("post");
        assert_eq!(response.status(), 202);
        assert_eq!(response.body_text_lossy(), "accepted");
    }
}
