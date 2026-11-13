//! Unit tests for `serenade-view`.

use serenade_http::{Method, Route, RouteCollection};

use crate::{
    AssetConfig, Document, DocumentParts, DocumentTemplate, RenderedDocument, ViewError, asset,
    escape_html, partial, path, version,
};

#[test]
fn crate_version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn path_wraps_route_generate() {
    let mut routes = RouteCollection::new();
    routes
        .add(Route::with_method("item", "/items/{id}", Method::Get))
        .expect("add");
    assert_eq!(
        path(&routes, "item", &[("id", "9")]).expect("path"),
        "/items/9"
    );
}

#[test]
fn asset_joins_and_rejects_unsafe() {
    assert_eq!(asset("myfeed.css").expect("ok"), "/assets/myfeed.css");
    assert_eq!(
        asset("myfeed.css?v=5").expect("query"),
        "/assets/myfeed.css?v=5"
    );
    assert_eq!(
        AssetConfig::with_base("/static")
            .join("js/app.js")
            .expect("ok"),
        "/static/js/app.js"
    );
    assert_eq!(asset("").expect("base"), "/assets");
    assert!(asset("../secret").is_err());
    assert!(asset("http://evil.example/x").is_err());
    assert!(asset("a//b").is_err());
    assert!(
        AssetConfig::with_base("cdn")
            .join("x")
            .expect("rel")
            .starts_with('/')
    );
    assert_eq!(
        AssetConfig::with_base("").join("x").expect("blank base"),
        "/x"
    );
}

#[test]
fn partial_runs_fragment() {
    let html = partial(|| format!("<span>{}</span>", escape_html("<ok>")));
    assert_eq!(html, "<span>&lt;ok&gt;</span>");
}

struct TestDocumentTemplate;

impl DocumentTemplate for TestDocumentTemplate {
    type Head = &'static str;
    type Body = &'static str;

    fn render(&self, parts: DocumentParts<Self::Head, Self::Body>) -> Result<String, ViewError> {
        Ok(format!(
            "<!doctype html><html lang=\"en\"><head>{}</head><body>{}</body></html>",
            parts.head, parts.body
        ))
    }
}

#[test]
fn document_composes_once_for_http_or_static_delivery() {
    let rendered = Document::new("<title>Page</title>", "<main>Content</main>")
        .render(&TestDocumentTemplate)
        .expect("render document");
    let expected = "<!doctype html><html lang=\"en\"><head><title>Page</title></head><body><main>Content</main></body></html>";
    assert_eq!(rendered.as_html(), expected);
    assert_eq!(rendered.clone().into_static_output(), expected);

    let response = rendered.into_response();
    assert_eq!(response.status(), 200);
    assert_eq!(response.body_str(), Some(expected));

    let response: RenderedDocument = Document::new("<title>Page</title>", "<main>Content</main>")
        .render(&TestDocumentTemplate)
        .expect("render document");
    let response = response.into_response_with_status(201);
    assert_eq!(response.status(), 201);
    assert_eq!(
        response.headers().get("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(response.body_str(), Some(expected));
}

#[cfg(feature = "leptos-ssr")]
mod leptos_document_tests {
    use leptos::prelude::*;

    use crate::{Document, initialize_leptos_ssr};
    use std::sync::Once;

    fn initialize_executor() {
        static INITIALIZE: Once = Once::new();
        INITIALIZE.call_once(|| {
            initialize_leptos_ssr().expect("initialize SSR executor");
        });
    }

    fn custom_document(head: Children, body: Children) -> impl IntoView {
        view! {
            <!DOCTYPE html>
            <html lang="fr">
                <head>{head()}</head>
                <body>{body()}</body>
            </html>
        }
    }

    #[component]
    fn RequestHead() -> impl IntoView {
        let request = use_context::<String>().expect("request context");
        view! {
            <title>{request}</title>
            <link rel="stylesheet" href="/assets/app.css" />
        }
    }

    #[component]
    fn RequestBody() -> impl IntoView {
        let request = use_context::<String>().expect("request context");
        let request_attribute = request.clone();
        let count = RwSignal::new(0);
        view! {
            <main data-request=request_attribute>
                <button on:click=move |_| count.update(|value| *value += 1)>
                    {move || count.get()}
                </button>
                {request}
            </main>
            <script type="module" src="/assets/app.js"></script>
        }
    }

    fn render_request(request: &'static str) -> String {
        initialize_executor();
        Document::new(RequestHead, RequestBody)
            .render_with_leptos(move |head, body| {
                provide_context(request.to_owned());
                custom_document(head, body)
            })
            .expect("render request")
            .into_static_output()
    }

    #[component]
    fn RegisteredMetadata() -> impl IntoView {
        view! {
            <crate::ManagedTitle text="Default title" />
            <crate::ManagedTitle text="Route & <title>" />
            <crate::ManagedMeta
                head_key="meta:name:description"
                name="description"
                content="Description & <details>"
            />
            <crate::ManagedLink
                head_key="link:canonical"
                rel="canonical"
                href="/products/shared"
            />
            <crate::ManagedLink
                head_key="link:canonical"
                rel="canonical"
                href="/products/one?a=1&b=2"
            />
            <crate::ManagedMeta
                head_key="meta:property:og:title"
                property="og:title"
                content="Product & <details>"
            />
            <crate::ManagedMeta property="og:image" content="/images/one.jpg" />
            <crate::ManagedMeta property="og:image" content="/images/two.jpg" />
            <crate::ManagedMeta
                head_key="meta:name:twitter:card"
                name="twitter:card"
                content="summary_large_image"
            />
            <crate::ManagedMeta
                head_key="meta:name:twitter:title"
                name="twitter:title"
                content="Product title"
            />
            <link rel="stylesheet" href="/assets/site.css" />
            <crate::JsonLd json=r#"{"name":"</script><img>"}"# />
        }
    }

    #[component]
    fn EscapedMetadata() -> impl IntoView {
        let title = r#"Crème "citée" & <été>"#;
        let description = r#"Résumé "vif" & <détail> - 日本語"#;
        let canonical = r#"/recherche?q="été"&lang=fr"#;
        let json = r#"{"name":"Crème \"citée\" & <été> </script><script>"}"#;
        view! {
            <crate::ManagedTitle text=title />
            <crate::ManagedMeta
                head_key="meta:name:description"
                name="description"
                content=description
            />
            <crate::ManagedLink
                head_key="link:canonical"
                rel="canonical"
                href=canonical
            />
            <crate::ManagedMeta
                head_key="meta:property:og:description"
                property="og:description"
                content=description
            />
            <crate::ManagedMeta
                head_key="meta:name:twitter:description"
                name="twitter:description"
                content=description
            />
            <crate::JsonLd json=json />
            <main>"Initial body"</main>
        }
    }

    #[component]
    fn HydrationProbe() -> impl IntoView {
        view! {
            <main>
                <button id="hydrated-action" on:click=move |_| ()>
                    "Hydrated"
                </button>
            </main>
        }
    }

    #[test]
    fn default_document_renders_framework_views_in_one_complete_shell() {
        initialize_executor();
        let rendered = Document::new(
            || {
                view! {
                    <title>"Home & <About>"</title>
                    <link rel="stylesheet" href="/assets/app.css" />
                }
            },
            || {
                view! {
                    <main>"<welcome>"</main>
                    <script type="module" src="/assets/app.js"></script>
                }
            },
        )
        .render_leptos()
        .expect("render document")
        .into_static_output();

        assert_eq!(
            rendered
                .to_ascii_lowercase()
                .matches("<!doctype html>")
                .count(),
            1
        );
        assert_eq!(rendered.matches("<html").count(), 1);
        assert_eq!(rendered.matches("<head").count(), 1);
        assert_eq!(rendered.matches("<body").count(), 1);
        assert!(rendered.contains("<html lang=\"en\">"));
        assert!(rendered.contains("<title>Home &amp; &lt;About&gt;</title>"));
        assert!(rendered.contains("<main>&lt;welcome&gt;</main>"));
        assert!(rendered.contains("href=\"/assets/app.css\""));
        assert!(rendered.contains("src=\"/assets/app.js\""));
    }

    #[test]
    fn managed_metadata_is_injected_into_head_and_preserves_repeatable_entries() {
        initialize_executor();
        let rendered = Document::new(RegisteredMetadata, || view! { <main>"Product"</main> })
            .render_leptos()
            .expect("render document")
            .into_static_output();
        let head = rendered
            .split_once("<head>")
            .expect("document head")
            .1
            .split_once("</head>")
            .expect("document head end")
            .0;

        assert!(head.contains("<title>Route &amp; &lt;title&gt;</title>"));
        assert_eq!(head.matches("<title>").count(), 1);
        assert!(head.contains("name=\"description\""));
        assert!(head.contains("content=\"Description &amp; &lt;details&gt;\""));
        assert!(head.contains("rel=\"canonical\""));
        assert!(head.contains("href=\"/products/one?a=1&amp;b=2\""));
        assert_eq!(head.matches("rel=\"canonical\"").count(), 1);
        assert_eq!(head.matches("property=\"og:image\"").count(), 2);
        assert!(head.contains("property=\"og:title\""));
        assert!(head.contains("content=\"Product &amp; &lt;details&gt;\""));
        assert!(head.contains("name=\"twitter:card\""));
        assert!(head.contains("content=\"summary_large_image\""));
        assert!(head.contains("name=\"twitter:title\""));
        assert!(head.contains("href=\"/assets/site.css\""));
        assert!(head.contains(r#"<script type="application/ld+json" data-serenade-head="ssr">{"name":"\u003c/script\u003e\u003cimg\u003e"}</script>"#));
        assert_eq!(head.matches("data-serenade-head=\"ssr\"").count(), 8);
        assert!(head.contains("<!--HEAD-->"));
        assert!(
            !rendered
                .split_once("<body>")
                .expect("document body")
                .1
                .contains("<title>")
        );
    }

    #[test]
    fn escapes_unicode_quotes_markup_and_script_breaking_metadata() {
        initialize_executor();
        let rendered = Document::new(EscapedMetadata, || view! { <main>"Page"</main> })
            .render_leptos()
            .expect("render document")
            .into_static_output();
        let head = rendered
            .split_once("<head>")
            .expect("document head")
            .1
            .split_once("</head>")
            .expect("document head end")
            .0;

        assert!(
            head.contains("<title>Crème &quot;citée&quot; &amp; &lt;été&gt;</title>"),
            "{head}"
        );
        assert!(head.contains(
            "name=\"description\" content=\"Résumé &quot;vif&quot; &amp; &lt;détail&gt; - 日本語\""
        ));
        assert!(head.contains("href=\"/recherche?q=&quot;été&quot;&amp;lang=fr\""));
        assert!(head.contains("property=\"og:description\""));
        assert!(head.contains("name=\"twitter:description\""));
        assert!(head.contains(
            r#"{"name":"Crème \"citée\" \u0026 \u003cété\u003e \u003c/script\u003e\u003cscript\u003e"}"#
        ));
        assert!(!head.contains("</script><script>"));
        assert!(rendered.contains("<body><main>Page</main></body>"));
    }

    #[test]
    fn hydration_browser_fixture_matches_server_rendered_body() {
        initialize_executor();
        let rendered = Document::new(
            || view! { <title>"Hydration fixture"</title> },
            HydrationProbe,
        )
        .render_leptos()
        .expect("render hydration fixture")
        .into_static_output();
        let body = rendered
            .split_once("<body>")
            .expect("document body")
            .1
            .split_once("</body>")
            .expect("document body end")
            .0;

        assert_eq!(
            body,
            include_str!("../tests/fixtures/hydration.html").trim_end()
        );
    }

    #[test]
    fn custom_document_projects_slots_and_preserves_request_hydration_context() {
        let rendered = render_request("request-one");

        assert!(rendered.contains("<html lang=\"fr\">"));
        assert!(rendered.contains("<title>request-one</title>"));
        assert!(rendered.contains("data-request=\"request-one\""));
        assert!(rendered.contains("<link rel=\"stylesheet\" href=\"/assets/app.css\">"));
        assert!(rendered.contains("<script type=\"module\" src=\"/assets/app.js\"></script>"));
    }

    #[test]
    fn concurrent_documents_have_isolated_request_state() {
        let first = std::thread::spawn(|| render_request("first"));
        let second = std::thread::spawn(|| render_request("second"));
        let first = first.join().expect("first render");
        let second = second.join().expect("second render");

        assert!(first.contains("data-request=\"first\""));
        assert!(!first.contains("data-request=\"second\""));
        assert!(second.contains("data-request=\"second\""));
        assert!(!second.contains("data-request=\"first\""));
    }
}
