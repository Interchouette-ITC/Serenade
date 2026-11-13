use leptos::prelude::*;
use rangular_aot::HostCell;
use serenade_view::{Document, ViewError};

use crate::{assets::CmsBootstrap, cms_hello_host::CmsHelloHost};

mod cms_hello {
    use leptos::prelude::*;

    include!(concat!(env!("OUT_DIR"), "/cms_hello.rs"));
}

pub fn cms_hello_page(route_specific: bool, bootstrap: &CmsBootstrap) -> Result<String, ViewError> {
    let host = HostCell::new(CmsHelloHost::new(route_specific));
    let head_host = host.clone();
    let document_host = host.clone();
    let document = Document::new(
        move || cms_hello::cms_head(head_host),
        move || cms_hello::cms_body(host),
    );
    let bootstrap = bootstrap.clone();
    let rendered = document
        .render_with_leptos(move |head, body| {
            let bootstrap: Children = Box::new(move || bootstrap_view(bootstrap).into_any());
            view! {
                <!DOCTYPE html>
                {cms_hello::cms_document(document_host, head, bootstrap, body)}
            }
        })?
        .into_static_output();
    Ok(rendered)
}

fn bootstrap_view(bootstrap: CmsBootstrap) -> impl IntoView {
    view! {
        <script
            type="module"
            src=bootstrap.script_url
            data-serenade-universal-bootstrap="true"
            data-wasm-js=bootstrap.js_url
            data-wasm-url=bootstrap.wasm_url
            data-init-with-object=bootstrap.init_with_object.to_string()
            data-import-bindings=bootstrap.import_bindings.to_string()
            data-import-bindings-name=bootstrap.import_bindings_name
            integrity=bootstrap.integrity
            crossorigin=bootstrap.crossorigin
        ></script>
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Once;

    use super::*;
    use crate::assets::CmsBootstrap;
    use serenade_view::initialize_leptos_ssr;

    fn test_bootstrap() -> CmsBootstrap {
        CmsBootstrap {
            script_url: "/universal-bootstrap.js".to_owned(),
            js_url: "/serenade-demo-front.js".to_owned(),
            wasm_url: "/serenade-demo-front_bg.wasm".to_owned(),
            init_with_object: true,
            import_bindings: true,
            import_bindings_name: "wasmBindings".to_owned(),
            integrity: None,
            crossorigin: None,
        }
    }

    #[test]
    fn cms_page_renders_complete_aot_document_with_escaped_route_metadata() {
        static INITIALIZE: Once = Once::new();
        INITIALIZE.call_once(|| {
            initialize_leptos_ssr().expect("initialize Leptos SSR");
        });

        let bootstrap = test_bootstrap();
        let page = cms_hello_page(true, &bootstrap).expect("render CMS Hello document");
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
        assert!(head.contains(
            r#"<script type="module" src="/universal-bootstrap.js" data-serenade-universal-bootstrap="true" data-wasm-js="/serenade-demo-front.js" data-wasm-url="/serenade-demo-front_bg.wasm" data-init-with-object="true" data-import-bindings="true" data-import-bindings-name="wasmBindings"></script>"#
        ));

        let body = page.split_once("<body>").expect("document body").1;
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

        let shared_page = cms_hello_page(false, &bootstrap).expect("render shared CMS document");
        assert!(shared_page.contains("<title>Serenade Universal</title>"));
        assert!(
            shared_page.contains("content=\"A shared description for the Universal document.\"")
        );
        assert!(shared_page.contains("href=\"/\""));
        assert!(!shared_page.contains("<title>Serenade CMS"));
        assert!(shared_page.contains("id=\"show-route\""));

        let mut request = crate::Request::new(crate::Method::Get, "/cms/hello");
        let response = crate::response(&mut request, std::path::Path::new("unused"), &bootstrap)
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
        let response = crate::response(&mut request, std::path::Path::new("unused"), &bootstrap)
            .expect("serve shared CMS route");
        assert!(
            response
                .body_str()
                .expect("shared HTML response body")
                .contains("<title>Serenade Universal</title>")
        );

        let mut request = crate::Request::new(crate::Method::Get, "/hello");
        let response = crate::response(&mut request, std::path::Path::new("unused"), &bootstrap)
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
