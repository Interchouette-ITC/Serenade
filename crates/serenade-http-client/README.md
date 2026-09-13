# serenade-http-client

Outbound HTTP client contract (`HttpClient`), an in-memory `MockHttpClient` for
tests, and an optional `ReqwestHttpClient` (Cargo feature `reqwest`, on by
default).

Inbound HTTP lives in `serenade-http`. This crate is for apps calling external APIs.
