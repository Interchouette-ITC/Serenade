//! Actix-web mounts for `OpenAPI` explorer UIs.

use actix_web::web;
use utoipa::openapi::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable as RedocServable};
use utoipa_scalar::{Scalar, Servable as ScalarServable};
use utoipa_swagger_ui::SwaggerUi;

use crate::OpenApiUiPaths;

/// Registers Swagger UI, Redoc, `RapiDoc`, and Scalar on `cfg`.
///
/// Call this on the Actix `App` **before** the Serenade kernel catch-all /
/// `default_service`. Explorers load the JSON at [`OpenApiUiPaths::openapi_json`].
///
/// The `OpenAPI` JSON route itself is app-owned (Serenade front controller or a
/// plain Actix handler) so products can keep dump/CI binaries independent of UI
/// features.
pub fn configure_actix_ui(cfg: &mut web::ServiceConfig, openapi: OpenApi, paths: &OpenApiUiPaths) {
    let swagger = paths.swagger_ui.clone();
    let redoc = paths.redoc.clone();
    let rapidoc = paths.rapidoc.clone();
    let scalar = paths.scalar.clone();
    let json = paths.openapi_json.clone();
    cfg.service(SwaggerUi::new(swagger).url(json.clone(), openapi.clone()))
        .service(Redoc::with_url(redoc, openapi.clone()))
        .service(RapiDoc::with_openapi(json, openapi.clone()).path(rapidoc))
        .service(Scalar::with_url(scalar, openapi));
}

#[cfg(test)]
mod tests {
    use actix_web::{App, test};
    use utoipa::OpenApi;

    use super::*;
    use crate::finalize_openapi;

    #[derive(OpenApi)]
    #[openapi(paths())]
    struct EmptyDoc;

    #[actix_web::test]
    async fn mounts_explorer_html_routes() {
        let mut doc = EmptyDoc::openapi();
        finalize_openapi(&mut doc, "test", "0.0.1", None);
        let paths = OpenApiUiPaths::new();
        let app = test::init_service(App::new().configure(|cfg| {
            configure_actix_ui(cfg, doc.clone(), &paths);
        }))
        .await;

        for uri in ["/redoc", "/rapidoc", "/scalar", "/swagger-ui/"] {
            let req = test::TestRequest::get().uri(uri).to_request();
            let resp = test::call_service(&app, req).await;
            assert!(resp.status().is_success(), "{uri} status={}", resp.status());
        }
    }
}
