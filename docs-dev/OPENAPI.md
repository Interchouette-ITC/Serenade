# OpenAPI (utoipa explorers)

**`serenade-openapi`** is the framework crate for OpenAPI document helpers and
optional explorer UIs. Applications still declare their own `#[derive(OpenApi)]`
paths and schemas; Serenade owns path conventions and UI mounting.

| Piece                                              | Role                                                                     |
| -------------------------------------------------- | ------------------------------------------------------------------------ |
| `OpenApiUiPaths`                                   | Default `/swagger-ui/`, `/redoc`, `/rapidoc`, `/scalar`, `/openapi.json` |
| `finalize_openapi` / `apply_info` / `apply_server` | Title, version, servers                                                  |
| Feature `actix`                                    | `configure_actix_ui` mounts all four explorers on Actix                  |

## App vs framework

| Owner           | Responsibility                                                                    |
| --------------- | --------------------------------------------------------------------------------- |
| **Application** | `#[derive(OpenApi)]` path list, `ToSchema` DTOs, serving `/openapi.json`, dump/CI |
| **Framework**   | Explorer mounts, default paths, Info/servers helpers, this document               |

OpenAPI is **opt-in**: depend on `serenade-openapi` and enable feature `actix` when
you want Swagger UI, Redoc, RapiDoc, and Scalar. Binaries that only need the JSON
contract do not require the UI feature.

## Actix beside HttpKernel listen

Mount explorers **before** the Serenade kernel catch-all:

```rust
use actix_web::{App, web};
use serenade_openapi::{OpenApiUiPaths, configure_actix_ui, finalize_openapi};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(/* app handlers */))]
struct ApiDoc;

fn app() -> App {
    let mut openapi = ApiDoc::openapi();
    finalize_openapi(
        &mut openapi,
        "my-service",
        env!("CARGO_PKG_VERSION"),
        Some("http://127.0.0.1:8080"),
    );
    let paths = OpenApiUiPaths::new();
    App::new()
        .configure(|cfg| configure_actix_ui(cfg, openapi, &paths))
        // app: GET /openapi.json from ApiDoc::openapi()
        // then: WebSocket routes, Serenade default_service / listen dispatch
}
```

Explorers load the JSON at `OpenApiUiPaths::openapi_json` (default `/openapi.json`).
Keep that route on the app so OpenAPI dump binaries stay independent of UI crates.

## Axum

No Axum mount helper in this crate. Axum apps use `finalize_openapi` and mount
utoipa UI crates themselves.

## Non-goals

- Generating commerce/domain schemas inside Serenade
- Replacing app `utoipa::path` annotations
- Mandating explorers in every Serenade binary
