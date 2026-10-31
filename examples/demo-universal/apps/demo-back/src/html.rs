use std::sync::Arc;

use leptos::prelude::*;

pub fn cms_hello_page() -> String {
    let owner = Owner::new_root(Some(Arc::new(hydration_context::SsrSharedContext::new())));

    owner.with(|| {
        view! {
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta charset="utf-8" />
                    <meta name="viewport" content="width=device-width, initial-scale=1" />
                    <title>"Serenade - CMS Hello"</title>
                    <link rel="icon" type="image/x-icon" href="/favicon.ico" />
                    <link rel="stylesheet" href="/style.css" />
                </head>
                <body>
                    <serenade_demo_ui::HelloUi />
                </body>
            </html>
        }
        .to_html()
            + r#"<script type="module">import init from "/serenade-demo-front.js"; await init({ module_or_path: "/serenade-demo-front_bg.wasm" });</script>"#
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cms_page_renders_shared_component_as_html_document() {
        let page = cms_hello_page();
        assert!(page.to_ascii_lowercase().starts_with("<!doctype html>"));
        assert!(page.contains("<h1>Hello from Serenade</h1>"));
        assert!(page.contains("src=\"/serenade-banner.png\""));
        assert!(page.contains("Clicked 0 times"));
    }
}
