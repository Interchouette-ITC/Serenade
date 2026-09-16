# serenade-openapi

Framework helpers for OpenAPI documents and explorer UIs.

- Always: `OpenApiUiPaths`, `apply_info` / `apply_server` / `finalize_openapi`
- Feature `actix`: `configure_actix_ui` mounts Swagger UI, Redoc, RapiDoc, and Scalar

Apps still own `#[derive(OpenApi)]` path/schema lists. See `docs-dev/OPENAPI.md`.
