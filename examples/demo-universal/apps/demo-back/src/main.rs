use std::path::PathBuf;

use serenade_demo_contract::HelloResponse;
use serenade_http::{AsyncHttpKernel, Method, Request, Response};
use serenade_kernel::{App, Application, Environment};
use serenade_minify::HtmlMinifier;
use serenade_view::{TRUNK_DIST_DIRECTORY_ENV, TrunkAssetManifest, initialize_leptos_ssr};

mod assets;
#[path = "../../../libs/cms_hello_host.rs"]
mod cms_hello_host;
mod html;

#[allow(clippy::needless_pass_by_ref_mut)]
fn response(
    request: &mut Request,
    dist: &std::path::Path,
    manifest: &TrunkAssetManifest,
    bootstrap: &assets::CmsBootstrap,
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
        (Method::Get, "/cms/hello") => {
            let body = html::cms_hello_page(
                cms_hello_host::route_specific_from_query(request.query()),
                bootstrap,
            )
            .map_err(|error| serenade_http::HttpError::status(500, error.to_string()))?;
            Ok(Response::new(200)
                .with_header("content-type", "text/html; charset=utf-8")
                .with_body(body.into_bytes()))
        }
        (Method::Get, path) => Ok(serve_asset(dist, manifest, path)),
        _ => Ok(Response::text(404, "Not found")),
    }
}

fn serve_asset(dist: &std::path::Path, manifest: &TrunkAssetManifest, uri: &str) -> Response {
    // Same URL mapping as `validate_output`, plus symlink containment for files the
    // manifest does not list; the public URL root is the HTML shell.
    let Ok(path) = manifest.resolve_output_file(dist, uri) else {
        return Response::text(404, "Not found");
    };
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
    initialize_leptos_ssr()?;
    let dist = std::env::var_os(TRUNK_DIST_DIRECTORY_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{TRUNK_DIST_DIRECTORY_ENV} is not set"))?;
    let manifest = TrunkAssetManifest::read_from_env()?;
    manifest.validate_output(&dist)?;
    let bootstrap = assets::CmsBootstrap::from_manifest(&manifest)?;
    let mut kernel = AsyncHttpKernel::from_sync(move |request: &mut Request| {
        response(request, &dist, &manifest, &bootstrap)
    });
    kernel.push_middleware(HtmlMinifier);
    let bind =
        std::env::var("SERENADE_UNIVERSAL_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    println!("demo-universal listening on http://{bind}");
    serenade_http_actix::listen(bind, kernel).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(public_url: &str) -> TrunkAssetManifest {
        TrunkAssetManifest::from_json(
            format!(
                r#"{{"version":2,"public_url":"{public_url}","html":"index.html","assets":[],"wasm":[]}}"#
            )
            .as_bytes(),
        )
        .expect("valid manifest")
    }

    #[test]
    fn assets_are_served_under_the_manifest_public_url() {
        let dist = std::env::temp_dir().join(format!(
            "serenade-demo-back-public-url-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dist).expect("dist");
        std::fs::write(dist.join("index.html"), "<html></html>").expect("html");
        std::fs::write(dist.join("app-123.js"), "export {}").expect("asset");

        let root = manifest("/");
        assert_eq!(serve_asset(&dist, &root, "/app-123.js").status(), 200);
        assert_eq!(serve_asset(&dist, &root, "/").status(), 200);

        for public_url in ["/app/", "/app"] {
            let nested = manifest(public_url);
            let served = serve_asset(&dist, &nested, "/app/app-123.js");
            assert_eq!(served.status(), 200, "{public_url}");
            assert_eq!(served.body(), b"export {}");
            for root in ["/app/", "/app"] {
                let shell = serve_asset(&dist, &nested, root);
                assert_eq!(shell.status(), 200, "{public_url} {root}");
                assert_eq!(shell.body(), b"<html></html>");
            }
            for outside in [
                "/",
                "/app-123.js",
                "/apple-123.js",
                "/app/../app-123.js",
                "/app/%2e%2e/x",
            ] {
                assert_eq!(
                    serve_asset(&dist, &nested, outside).status(),
                    404,
                    "{public_url} {outside}"
                );
            }
        }
        std::fs::remove_dir_all(&dist).expect("cleanup");
    }

    #[test]
    #[cfg(unix)]
    fn symlinks_out_of_dist_are_not_served() {
        let root =
            std::env::temp_dir().join(format!("serenade-demo-back-symlink-{}", std::process::id()));
        let dist = root.join("dist");
        std::fs::create_dir_all(&dist).expect("dist");
        std::fs::write(root.join("secret.txt"), "secret").expect("outside file");
        std::os::unix::fs::symlink(root.join("secret.txt"), dist.join("leak.txt"))
            .expect("symlink");
        let response = serve_asset(&dist, &manifest("/"), "/leak.txt");
        assert_eq!(response.status(), 404);
        assert_ne!(response.body(), b"secret");
        std::fs::remove_dir_all(&root).expect("cleanup");
    }
}
