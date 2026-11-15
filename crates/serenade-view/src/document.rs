//! Engine-neutral document composition and output delivery.

use serenade_http::Response;

use crate::ViewError;

#[cfg(feature = "leptos-ssr")]
use crate::head::ManagedHead;
#[cfg(feature = "leptos-ssr")]
use any_spawner::Executor as LeptosExecutor;
#[cfg(feature = "leptos-ssr")]
use futures::StreamExt;
#[cfg(feature = "leptos-ssr")]
use hydration_context::SsrSharedContext;
#[cfg(feature = "leptos-ssr")]
use leptos::prelude::*;
#[cfg(feature = "leptos-ssr")]
use leptos_meta::ServerMetaContext;
#[cfg(feature = "leptos-ssr")]
use std::sync::Arc;

/// Head and body inputs supplied to a document template.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentParts<Head, Body> {
    /// Application-owned head output or view factory.
    pub head: Head,
    /// Application-owned body output or view factory.
    pub body: Body,
}

impl<Head, Body> DocumentParts<Head, Body> {
    /// Creates document parts from the head and body inputs.
    #[must_use]
    pub const fn new(head: Head, body: Body) -> Self {
        Self { head, body }
    }
}

/// A template engine's document-level composition boundary.
///
/// Implementations own the document structure, language, and head/body slots.
/// They consume the head and body inputs and return one complete HTML document.
///
/// The application head template owns title, metadata, canonical links, JSON-LD,
/// and custom head markup; the body template/component owns page content. Shared
/// head entries come from the document template. Route-owned entries replace
/// shared singleton entries only when they declare the same stable key, leaving
/// unrelated shared entries intact. Repeatable entries use distinct keys and are
/// retained. Keys are declared by templates/integrations, never recovered by
/// parsing rendered HTML.
///
/// Rust controllers remain responsible for data and orchestration, not markup.
/// Template-engine and AOT adapters can implement this contract as optional
/// integrations; the engine-neutral `serenade-view` helpers do not require them.
pub trait DocumentTemplate<Head, Body> {
    /// Composes the complete HTML document.
    ///
    /// # Errors
    ///
    /// Returns an error if the template engine cannot render the document.
    fn render(self, parts: DocumentParts<Head, Body>) -> Result<String, ViewError>;
}

/// The framework's default document template.
///
/// It composes a complete English-language HTML document from already-rendered
/// strings or, with the `leptos-ssr` feature, from view factories.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultDocumentTemplate;

impl<Head: AsRef<str>> DocumentTemplate<Head, String> for DefaultDocumentTemplate {
    fn render(self, parts: DocumentParts<Head, String>) -> Result<String, ViewError> {
        Ok(default_document_html(parts.head.as_ref(), &parts.body))
    }
}

impl<Head: AsRef<str>> DocumentTemplate<Head, &str> for DefaultDocumentTemplate {
    fn render(self, parts: DocumentParts<Head, &str>) -> Result<String, ViewError> {
        Ok(default_document_html(parts.head.as_ref(), parts.body))
    }
}

fn default_document_html(head: &str, body: &str) -> String {
    format!("<!DOCTYPE html><html lang=\"en\"><head>{head}</head><body>{body}</body></html>")
}

/// Build-generated markup delivered with every rendered document.
///
/// Typically obtained once at startup from
/// [`TrunkAssetManifest::document_assets`](crate::TrunkAssetManifest::document_assets).
/// The markup is trusted build output and is inserted verbatim: head markup
/// immediately before the document's `</head>`, body markup immediately before
/// its `</body>`. Document templates must not repeat these tags themselves.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocumentAssets {
    head: String,
    body: String,
}

impl DocumentAssets {
    /// Creates document assets from trusted head and body markup.
    #[must_use]
    pub fn new(head: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            head: head.into(),
            body: body.into(),
        }
    }

    /// Markup inserted at the end of the document head.
    #[must_use]
    pub fn head_html(&self) -> &str {
        &self.head
    }

    /// Markup inserted at the end of the document body.
    #[must_use]
    pub fn body_html(&self) -> &str {
        &self.body
    }

    /// Whether there is no markup to insert.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.head.is_empty() && self.body.is_empty()
    }

    fn insert_into(&self, html: String) -> Result<String, ViewError> {
        if self.is_empty() {
            return Ok(html);
        }
        let mut html = html;
        if !self.head.is_empty() {
            let index = find_ascii_case_insensitive(&html, "</head>", false).ok_or_else(|| {
                ViewError::msg("rendered document has no </head> for its head assets")
            })?;
            html.insert_str(index, &self.head);
        }
        if !self.body.is_empty() {
            let index = find_ascii_case_insensitive(&html, "</body>", true).ok_or_else(|| {
                ViewError::msg("rendered document has no </body> for its body assets")
            })?;
            html.insert_str(index, &self.body);
        }
        Ok(html)
    }
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str, last: bool) -> Option<usize> {
    let (haystack, needle) = (haystack.as_bytes(), needle.as_bytes());
    let mut windows = haystack.windows(needle.len());
    let matches = |window: &[u8]| window.eq_ignore_ascii_case(needle);
    if last {
        windows.rposition(matches)
    } else {
        windows.position(matches)
    }
}

/// Application-provided head and body ready for document composition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document<Head, Body> {
    parts: DocumentParts<Head, Body>,
    assets: DocumentAssets,
}

impl<Head, Body> Document<Head, Body> {
    /// Starts document composition with application-owned head and body outputs.
    #[must_use]
    pub const fn new(head: Head, body: Body) -> Self {
        Self {
            parts: DocumentParts::new(head, body),
            assets: DocumentAssets {
                head: String::new(),
                body: String::new(),
            },
        }
    }

    /// Delivers build-generated assets, such as Trunk's stylesheets, scripts
    /// and WASM bootstrap, with the rendered document.
    ///
    /// # Examples
    ///
    /// ```
    /// use serenade_view::{Document, DocumentAssets};
    ///
    /// let assets = DocumentAssets::new(
    ///     r#"<link rel="stylesheet" href="/app-1a2b.css">"#,
    ///     r#"<script type="module" src="/app-1a2b.js"></script>"#,
    /// );
    /// let rendered = Document::new("<title>Home</title>", "<main>Welcome</main>")
    ///     .with_assets(&assets)
    ///     .render()?;
    /// assert!(rendered.as_html().contains(r#"href="/app-1a2b.css"></head>"#));
    /// assert!(rendered.as_html().contains(r#"</script></body>"#));
    /// # Ok::<(), serenade_view::ViewError>(())
    /// ```
    #[must_use]
    pub fn with_assets(mut self, assets: &DocumentAssets) -> Self {
        self.assets = assets.clone();
        self
    }

    /// Renders the document through the framework's default template.
    ///
    /// String inputs must already be rendered and escaped; they are not sanitized.
    /// View factories require the `leptos-ssr` feature and a process-wide SSR
    /// executor initialized with `initialize_leptos_ssr` or by the application.
    ///
    /// # Examples
    ///
    /// ```
    /// use serenade_view::Document;
    ///
    /// let document = Document::new("<title>Home</title>", "<main>Welcome</main>");
    /// let rendered = document.render()?;
    /// assert!(rendered.as_html().contains("<main>Welcome</main>"));
    /// # Ok::<(), serenade_view::ViewError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the default template's rendering error, or an asset insertion
    /// error as described for [`Self::render_with`].
    pub fn render(self) -> Result<RenderedDocument, ViewError>
    where
        DefaultDocumentTemplate: DocumentTemplate<Head, Body>,
    {
        self.render_with(DefaultDocumentTemplate)
    }

    /// Renders the document through an application-provided template.
    ///
    /// With the `leptos-ssr` feature, a template can be a function or closure
    /// accepting the head and body `DocumentSlot`s. Both slots and the
    /// template render under the same request-scoped owner. The application
    /// must initialize the SSR executor before rendering view factories.
    ///
    /// # Examples
    ///
    /// ```
    /// use serenade_view::{Document, DocumentParts, DocumentTemplate, ViewError};
    ///
    /// struct FrenchTemplate;
    ///
    /// impl DocumentTemplate<&str, &str> for FrenchTemplate {
    ///     fn render(self, parts: DocumentParts<&str, &str>) -> Result<String, ViewError> {
    ///         Ok(format!(
    ///             "<!DOCTYPE html><html lang=\"fr\"><head>{}</head><body>{}</body></html>",
    ///             parts.head, parts.body
    ///         ))
    ///     }
    /// }
    ///
    /// let document = Document::new("<title>Accueil</title>", "<main>Bienvenue</main>");
    /// let rendered = document.render_with(FrenchTemplate)?;
    /// assert!(rendered.as_html().contains("<html lang=\"fr\">"));
    /// # Ok::<(), ViewError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the template's rendering error, or an error when assets are
    /// configured and the rendered document lacks `</head>` or `</body>`.
    pub fn render_with(
        self,
        template: impl DocumentTemplate<Head, Body>,
    ) -> Result<RenderedDocument, ViewError> {
        let html = template.render(self.parts)?;
        self.assets.insert_into(html).map(RenderedDocument)
    }
}

#[cfg(feature = "leptos-ssr")]
/// A one-shot head or body slot passed to a document template.
pub type DocumentSlot = Children;

#[cfg(feature = "leptos-ssr")]
impl<Head, Body, HeadView, BodyView> DocumentTemplate<Head, Body> for DefaultDocumentTemplate
where
    Head: FnOnce() -> HeadView + Send + 'static,
    Body: FnOnce() -> BodyView + Send + 'static,
    HeadView: IntoView + 'static,
    BodyView: IntoView + 'static,
{
    fn render(self, parts: DocumentParts<Head, Body>) -> Result<String, ViewError> {
        render_view_document(parts, |head, body| {
            view! {
                <!DOCTYPE html>
                <html lang="en">
                    <head>{head()}</head>
                    <body>{body()}</body>
                </html>
            }
        })
    }
}

#[cfg(feature = "leptos-ssr")]
impl<Head, Body, HeadView, BodyView, Template, View> DocumentTemplate<Head, Body> for Template
where
    Head: FnOnce() -> HeadView + Send + 'static,
    Body: FnOnce() -> BodyView + Send + 'static,
    HeadView: IntoView + 'static,
    BodyView: IntoView + 'static,
    Template: FnOnce(DocumentSlot, DocumentSlot) -> View,
    View: RenderHtml,
{
    fn render(self, parts: DocumentParts<Head, Body>) -> Result<String, ViewError> {
        render_view_document(parts, self)
    }
}

#[cfg(feature = "leptos-ssr")]
fn render_view_document<Head, Body, HeadView, BodyView, View>(
    parts: DocumentParts<Head, Body>,
    template: impl FnOnce(DocumentSlot, DocumentSlot) -> View,
) -> Result<String, ViewError>
where
    Head: FnOnce() -> HeadView + Send + 'static,
    Body: FnOnce() -> BodyView + Send + 'static,
    HeadView: IntoView + 'static,
    BodyView: IntoView + 'static,
    View: RenderHtml,
{
    let (meta_context, meta_output) = ServerMetaContext::new();
    let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new())));
    let DocumentParts { head, body } = parts;
    let html = owner.with(move || {
        provide_context(meta_context);
        let head: DocumentSlot =
            Box::new(move || view! { <ManagedHead>{head()}</ManagedHead> }.into_any());
        let body: DocumentSlot = Box::new(move || body().into_any());
        template(head, body).to_html()
    });
    owner.cleanup();
    futures::executor::block_on(async move {
        let input = futures::stream::iter([html]);
        let mut output = Box::pin(meta_output.inject_meta_context(input).await);
        output
            .next()
            .await
            .ok_or_else(|| ViewError::msg("metadata injection produced no document chunk"))
    })
}

#[cfg(feature = "leptos-ssr")]
/// Initializes Leptos SSR with the futures executor.
///
/// Applications that use a different runtime should initialize
/// [`LeptosExecutor`] directly instead.
///
/// # Errors
///
/// Returns an error if another executor has already been initialized.
pub fn initialize_leptos_ssr() -> Result<(), ViewError> {
    LeptosExecutor::init_futures_executor().map_err(|error| {
        ViewError::msg(format!(
            "unable to initialize the Leptos SSR executor: {error:?}"
        ))
    })
}

/// A complete HTML document ready for HTTP or static-file delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedDocument(String);

impl RenderedDocument {
    /// Returns the complete HTML document.
    #[must_use]
    pub fn as_html(&self) -> &str {
        &self.0
    }

    /// Delivers the document as an HTTP 200 response with an HTML content type.
    #[must_use]
    pub fn into_response(self) -> Response {
        self.into_response_with_status(200)
    }

    /// Delivers the document as an HTTP response with the supplied status.
    #[must_use]
    pub fn into_response_with_status(self, status: u16) -> Response {
        Response::new(status)
            .with_header("content-type", "text/html; charset=utf-8")
            .with_body(self.0.into_bytes())
    }

    /// Returns the document for static-output writing.
    #[must_use]
    pub fn into_static_output(self) -> String {
        self.0
    }
}
