use std::path::PathBuf;

use serenade_demo_contract::HelloResponse;
use serenade_http::{AsyncHttpKernel, Method, Request, Response};
use serenade_kernel::{App, Application, Environment};

#[allow(clippy::needless_pass_by_ref_mut)]
fn response(
    request: &mut Request,
    dist: &std::path::Path,
) -> Result<Response, serenade_http::HttpError> {
    match (request.method(), request.path()) {
        (Method::Get, "/hello") => {
            let body = serde_json::to_vec(&HelloResponse {
                message: "Hello from the Serenade backend!".to_owned(),
            })
            .map_err(|error| serenade_http::HttpError::status(500, error.to_string()))?;
            Ok(Response::new(200)
                .with_header("content-type", "application/json; charset=utf-8")
                .with_body(body))
        }
        (Method::Get, "/cms/hello") => Ok(Response::new(200)
            .with_header("content-type", "text/html; charset=utf-8")
            .with_body(b"<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Serenade</title><link rel=\"stylesheet\" href=\"/style.css\"><body><main class=\"hello\"><header class=\"hello__header\"><img class=\"hello__banner\" src=\"/serenade-banner.png\" alt=\"Serenade\" width=\"534\" height=\"147\"></header><section class=\"hello__content\"><h1>Hello from Serenade</h1><p>This page is rendered live by the backend.</p><a href=\"/\">Back to the app</a></section></main></body></html>".to_vec())),
        (Method::Get, path) => Ok(serve_asset(dist, path)),
        _ => Ok(Response::text(404, "Not found")),
    }
}

fn serve_asset(dist: &std::path::Path, uri: &str) -> Response {
    let relative = uri.trim_start_matches('/');
    let requested = if relative.is_empty() {
        "index.html"
    } else {
        relative
    };
    if requested.split('/').any(|part| part == "." || part == "..") {
        return Response::text(400, "Invalid asset path");
    }
    let path = dist.join(requested);
    let Ok(bytes) = std::fs::read(&path) else {
        return Response::text(404, "Not found");
    };
    let content_type = match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("ico") => "image/x-icon",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    };
    Response::new(200)
        .with_header("content-type", content_type)
        .with_body(bytes)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(Environment::from_name("dev")?);
    app.boot()?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dist = root.join("../../apps/demo-front/dist");
    let kernel = AsyncHttpKernel::from_sync(move |request: &mut Request| response(request, &dist));
    let bind =
        std::env::var("SERENADE_UNIVERSAL_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    println!("demo-universal listening on http://{bind}");
    serenade_http_actix::listen(bind, kernel).await?;
    Ok(())
}
