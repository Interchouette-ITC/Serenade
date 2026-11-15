//! Public document composition API.

use serenade_view::{Document, DocumentParts, DocumentTemplate, ViewError};

struct PageTemplate<'a> {
    language: &'a str,
}

impl<'head, 'body> DocumentTemplate<&'head str, &'body str> for PageTemplate<'_> {
    fn render(self, parts: DocumentParts<&'head str, &'body str>) -> Result<String, ViewError> {
        Ok(format!(
            "<!DOCTYPE html><html lang=\"{}\"><head>{}</head><body>{}</body></html>",
            self.language, parts.head, parts.body
        ))
    }
}

#[test]
fn rendered_strings_support_default_and_borrowed_custom_templates() {
    let head = String::from("<title>Home</title>");
    let body = String::from("<main>Welcome</main>");
    let language = String::from("fr");
    let document = Document::new(head.as_str(), body.as_str());

    let default = document.clone().render().expect("default document");
    let custom = document
        .render_with(PageTemplate {
            language: &language,
        })
        .expect("custom document");

    assert!(default.as_html().contains("<html lang=\"en\">"));
    assert!(custom.as_html().contains("<html lang=\"fr\">"));
    assert!(default.as_html().contains("<main>Welcome</main>"));
    assert!(custom.as_html().contains("<main>Welcome</main>"));
}

#[cfg(feature = "leptos-ssr")]
mod view_templates {
    use std::sync::Once;

    use leptos::prelude::*;
    use serenade_view::{Document, DocumentSlot, ManagedTitle, initialize_leptos_ssr};

    fn head() -> impl IntoView {
        view! { <ManagedTitle text="Home" /> }
    }

    fn body() -> impl IntoView {
        view! { <main>"Welcome"</main> }
    }

    fn template(head: DocumentSlot, body: DocumentSlot) -> impl IntoView {
        view! {
            <!DOCTYPE html>
            <html lang="en">
                <head>{head()}</head>
                <body>{body()}</body>
            </html>
        }
    }

    #[test]
    fn named_custom_template_uses_the_same_composition_as_the_default() {
        static INITIALIZE: Once = Once::new();
        INITIALIZE.call_once(|| initialize_leptos_ssr().expect("initialize SSR executor"));

        let default = Document::new(head, body)
            .render()
            .expect("default document");
        let custom = Document::new(head, body)
            .render_with(template)
            .expect("custom document");

        assert_eq!(default, custom);
        assert!(custom.as_html().contains("<title>Home</title>"));
        assert!(custom.as_html().contains("<main>Welcome</main>"));
    }
}
