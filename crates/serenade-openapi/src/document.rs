//! Info and servers helpers on a utoipa [`OpenApi`](utoipa::openapi::OpenApi) document.

use utoipa::openapi::{Info, OpenApi, Server};

/// Sets `info.title` and `info.version` on `openapi`.
pub fn apply_info(openapi: &mut OpenApi, title: impl Into<String>, version: impl Into<String>) {
    openapi.info = Info::new(title.into(), version.into());
}

/// Sets a single `servers` entry when `server_url` is non-empty after trim.
///
/// Pass the public base URL (for example `http://127.0.0.1:8080` or a reverse-proxy
/// prefix). Empty input clears servers.
pub fn apply_server(openapi: &mut OpenApi, server_url: impl AsRef<str>) {
    let url = server_url.as_ref().trim();
    if url.is_empty() {
        openapi.servers = None;
        return;
    }
    openapi.servers = Some(vec![Server::new(url)]);
}

/// Applies title, version, and optional server in one step.
pub fn finalize_openapi(
    openapi: &mut OpenApi,
    title: impl Into<String>,
    version: impl Into<String>,
    server_url: Option<&str>,
) {
    apply_info(openapi, title, version);
    match server_url {
        Some(url) => apply_server(openapi, url),
        None => openapi.servers = None,
    }
}

#[cfg(test)]
mod tests {
    use utoipa::OpenApi;

    use super::*;

    #[derive(OpenApi)]
    #[openapi(paths())]
    struct EmptyDoc;

    #[test]
    fn apply_info_and_server() {
        let mut doc = EmptyDoc::openapi();
        finalize_openapi(&mut doc, "demo", "0.1.0", Some("http://127.0.0.1:8080"));
        assert_eq!(doc.info.title, "demo");
        assert_eq!(doc.info.version, "0.1.0");
        let servers = doc.servers.expect("servers");
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].url, "http://127.0.0.1:8080");
    }

    #[test]
    fn empty_server_clears() {
        let mut doc = EmptyDoc::openapi();
        apply_server(&mut doc, "http://x");
        apply_server(&mut doc, "  ");
        assert!(doc.servers.is_none());
    }
}
