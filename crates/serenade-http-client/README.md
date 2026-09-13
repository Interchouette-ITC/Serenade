# serenade-http-client

Outbound HTTP client contract (`HttpClient` / `DynHttpClient`), an in-memory
`MockHttpClient` for tests, optional `ReqwestHttpClient` (Cargo feature
`reqwest`, on by default), and DI via `RegisterDefaultHttpClientPass`
(service id `http_client`).

Inbound HTTP lives in `serenade-http`. This crate is for apps calling external APIs.

See `docs-dev/HTTP_CLIENT.md`.
