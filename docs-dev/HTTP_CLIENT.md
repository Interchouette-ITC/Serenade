# HttpClient

Outbound HTTP lives in **`serenade-http-client`** ([#184](https://github.com/Interchouette-ITC/Serenade/issues/184), [#188](https://github.com/Interchouette-ITC/Serenade/issues/188), [#189](https://github.com/Interchouette-ITC/Serenade/issues/189)).

Inbound request/response types stay in **`serenade-http`**. This crate is the Symfony **HttpClient** analogue for webhooks, third-party APIs, and similar outbound calls.

## Types

| Type | Role |
| --- | --- |
| `ClientMethod` | GET / POST / PUT / PATCH / DELETE / HEAD |
| `ClientRequest` | Method, URL, headers, body, optional timeout |
| `ClientResponse` | Status, headers, body |
| `HttpClient` | Async `send` / `get` / `post` contract |
| `DynHttpClient` | Object-safe form for `Arc<dyn …>` in DI (`send_boxed`) |
| `HttpClientError` | Request / transport / mock-miss failures |

## Adapters

| Adapter | Role |
| --- | --- |
| `MockHttpClient` | Scripted expectations for unit tests |
| `ReqwestHttpClient` | Production client via reqwest + rustls (Cargo feature `reqwest`, on by default) |

## DI

`FrameworkExtension` adds [`RegisterDefaultHttpClientPass`], which registers service id `http_client` (`DEFAULT_HTTP_CLIENT_SERVICE`) tagged `http.client` with a `ReqwestHttpClient` when missing. Resolve `HttpClientService` and call `send` / `get` / `post`.

Apps replace the default by registering their own `http_client` service (for example `MockHttpClient` in tests) before compile.

## Example

```rust
use serenade_http_client::{ClientRequest, HttpClient, MockHttpClient, ClientMethod, ClientResponse};

let mock = MockHttpClient::new();
mock.expect(
    ClientMethod::Get,
    "https://example.test/ping",
    ClientResponse::new(200).body_text("pong"),
    Some(1),
);
let response = mock.get("https://example.test/ping").await?;
assert_eq!(response.body_text_lossy(), "pong");

let _ = ClientRequest::post("https://example.test/hook")
    .header("content-type", "application/json")
    .body(br#"{"ok":true}"#.to_vec());
```

Reqwest (feature `reqwest`):

```rust
use serenade_http_client::{HttpClient, ReqwestHttpClient};

let client = ReqwestHttpClient::new();
let response = client.get("https://example.test/").await?;
```

## Related

- Parent epic: [#184](https://github.com/Interchouette-ITC/Serenade/issues/184)
- Inbound HTTP: [KERNEL.md](KERNEL.md) (request path)
