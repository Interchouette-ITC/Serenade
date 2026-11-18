use leptos::prelude::*;
use rangular::aot::HostCell;
use serenade_view::{Document, DocumentSlot, ViewError};

use crate::{assets::CmsAssets, cms_hello_host::CmsHelloHost};

mod cms_hello {
    use leptos::prelude::*;

    include!(concat!(env!("OUT_DIR"), "/rangular/document_view.rs"));
    include!(concat!(env!("OUT_DIR"), "/rangular/head_view.rs"));
    include!(concat!(env!("OUT_DIR"), "/rangular/body_view.rs"));
}

pub fn cms_hello_page(route_specific: bool, assets: &CmsAssets) -> Result<String, ViewError> {
    let host = HostCell::new(CmsHelloHost::new(route_specific, &assets.public_url));
    let head_host = host.clone();
    let document_host = host.clone();
    let document = Document::new(
        move || cms_hello::head_view(head_host),
        move || cms_hello::body_view(host),
    );
    let rendered = document
        .with_assets(&assets.document)
        .render_with(move |head: DocumentSlot, body: DocumentSlot| {
            view! {
                <!DOCTYPE html>
                {cms_hello::document_view(document_host, head, body)}
            }
        })?
        .into_static_output();
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use super::*;
    use serenade_view::{DocumentAssets, initialize_leptos_ssr};

    const TRUNK_HEAD: &str = r#"<link rel="stylesheet" href="/style-1a.css"/><script type="module">import init from '/serenade-demo-front-2b.js';await init();</script>"#;
    const TRUNK_BODY: &str = "<svg data-trunk-inline></svg>";

    fn test_assets() -> CmsAssets {
        CmsAssets {
            public_url: "/".to_owned(),
            document: DocumentAssets::new(TRUNK_HEAD, TRUNK_BODY),
        }
    }

    fn initialize() {
        static INITIALIZE: Once = Once::new();
        INITIALIZE.call_once(|| {
            initialize_leptos_ssr().expect("initialize Leptos SSR");
        });
    }

    #[test]
    fn cms_page_resolves_frontend_urls_under_a_non_root_public_url() {
        initialize();
        let assets = CmsAssets {
            public_url: "/app".to_owned(),
            document: DocumentAssets::default(),
        };
        for route_specific in [true, false] {
            let page = cms_hello_page(route_specific, &assets).expect("render CMS document");
            for url in [
                "href=\"/app/favicon.ico\"",
                "content=\"/app/serenade-banner.png\"",
                "content=\"/app/serenade-banner.png?crop=cms&amp;width=1200\"",
                "src=\"/app/serenade-banner.png\"",
                "<a href=\"/app/\">Return to the Universal SPA</a>",
                "name=\"serenade-public-url\" content=\"/app/\"",
            ] {
                assert!(page.contains(url), "{url} in {page}");
            }
            for root_url in ["\"/favicon.ico", "\"/serenade-banner.png", "href=\"/\""] {
                assert!(!page.contains(root_url), "{root_url} in {page}");
            }
        }
        let shared = cms_hello_page(false, &assets).expect("render shared CMS document");
        assert!(
            shared.contains("rel=\"canonical\" href=\"/app/\"")
                || shared.contains("href=\"/app/\" rel=\"canonical\""),
            "{shared}"
        );
    }

    #[test]
    fn cms_page_renders_complete_aot_document_with_escaped_route_metadata() {
        initialize();

        let assets = test_assets();
        let manifest = serenade_view::TrunkAssetManifest::from_json(
            br#"{"public_url":"/","html":"index.html","assets":[],"wasm":[],"document_assets":[]}"#,
        )
        .expect("valid manifest");
        let page = cms_hello_page(true, &assets).expect("render CMS Hello document");
        let lower = page.to_ascii_lowercase();
        assert!(lower.starts_with("<!doctype html>"));
        assert_eq!(page.matches("<html").count(), 1);
        assert_eq!(page.matches("<head>").count(), 1);
        assert_eq!(page.matches("<body>").count(), 1);

        let head = page
            .split_once("<head>")
            .expect("document head")
            .1
            .split_once("</head>")
            .expect("document head end")
            .0;
        assert!(
            head.contains("<title>Serenade CMS &amp; &lt;Hello&gt;</title>"),
            "{head}"
        );
        assert_eq!(head.matches("<title>").count(), 1);
        assert_eq!(head.matches("data-serenade-head=\"ssr\"").count(), 9);
        assert!(!head.contains("A shared description"));
        assert!(head.contains("name=\"description\""));
        assert!(head.contains(
            "content=\"A request-time generated description with &amp; &lt;angle brackets&gt;.\""
        ));
        assert_eq!(head.matches("rel=\"canonical\"").count(), 1);
        assert!(head.contains("href=\"/cms/hello?source=universal&amp;mode=ssr\""));
        assert_eq!(head.matches("property=\"og:image\"").count(), 2);
        assert!(head.contains("content=\"/serenade-banner.png?crop=cms&amp;width=1200\""));
        assert!(head.contains(
            r#"<script type="application/ld+json" data-serenade-head="ssr">{"@context":"https://schema.org","name":"Serenade & \u003cUniversal>"}</script>"#
        ));
        assert!(head.ends_with(TRUNK_HEAD), "{head}");
        assert!(!page.contains("universal-bootstrap"));

        let body = page.split_once("<body>").expect("document body").1;
        assert!(
            body.ends_with(&format!("{TRUNK_BODY}</body></html>")),
            "{body}"
        );
        assert!(
            body.contains("<h1 id=\"cms-heading\">Hello from Rangular</h1>"),
            "{body}"
        );
        assert!(
            body.contains(
                "<p id=\"cms-message\">This dynamic message contains &amp; and &lt;angle brackets&gt;.</p>"
            )
        );
        assert!(
            body.contains("<button id=\"cms-counter\" type=\"button\">Clicked 0 times</button>")
        );
        assert!(body.contains("id=\"show-shared\""));
        assert!(body.contains("<a href=\"/\">Return to the Universal SPA</a>"));
        assert!(body.contains("<img"));

        let shared_page = cms_hello_page(false, &assets).expect("render shared CMS document");
        assert!(shared_page.contains("<title>Serenade Universal</title>"));
        assert!(
            shared_page.contains("content=\"A shared description for the Universal document.\"")
        );
        assert!(shared_page.contains("href=\"/\""));
        assert!(!shared_page.contains("<title>Serenade CMS"));
        assert!(shared_page.contains("id=\"show-route\""));

        let mut request = crate::Request::new(crate::Method::Get, "/cms/hello");
        let response = crate::response(
            &mut request,
            std::path::Path::new("unused"),
            &manifest,
            &assets,
        )
        .expect("serve CMS Hello route");
        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers().get("content-type"),
            Some("text/html; charset=utf-8")
        );
        assert!(
            response
                .body_str()
                .expect("HTML response body")
                .contains("<title>Serenade CMS &amp; &lt;Hello&gt;</title>")
        );

        let mut request =
            crate::Request::new(crate::Method::Get, "/cms/hello").with_query("view=shared");
        let response = crate::response(
            &mut request,
            std::path::Path::new("unused"),
            &manifest,
            &assets,
        )
        .expect("serve shared CMS route");
        assert!(
            response
                .body_str()
                .expect("shared HTML response body")
                .contains("<title>Serenade Universal</title>")
        );

        let mut request = crate::Request::new(crate::Method::Get, "/hello");
        let response = crate::response(
            &mut request,
            std::path::Path::new("unused"),
            &manifest,
            &assets,
        )
        .expect("serve existing hello API route");
        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers().get("content-type"),
            Some("application/json; charset=utf-8")
        );
        assert_eq!(
            response.body_str(),
            Some(r#"{"message":"Hello from the Serenade backend!"}"#)
        );
    }
}
