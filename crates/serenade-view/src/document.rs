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

/// Rendered head and body inputs supplied to a document template.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentParts<Head, Body> {
    /// Application-owned head template output.
    pub head: Head,
    /// Application-owned body template or component output.
    pub body: Body,
}

impl<Head, Body> DocumentParts<Head, Body> {
    /// Creates document parts from the head and body template outputs.
    #[must_use]
    pub const fn new(head: Head, body: Body) -> Self {
        Self { head, body }
    }
}

/// A template engine's document-level composition boundary.
///
/// Implementations own the document structure, language, and head/body slots.
/// They receive already-rendered head and body template outputs and return one
/// complete HTML document.
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
pub trait DocumentTemplate {
    /// The rendered head template output type.
    type Head;
    /// The rendered body template or component output type.
    type Body;

    /// Composes the complete HTML document.
    ///
    /// # Errors
    ///
    /// Returns an error if the template engine cannot render the document.
    fn render(&self, parts: DocumentParts<Self::Head, Self::Body>) -> Result<String, ViewError>;
}

/// Application-provided head and body ready for document composition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document<Head, Body> {
    parts: DocumentParts<Head, Body>,
}

impl<Head, Body> Document<Head, Body> {
    /// Starts document composition with application-owned head and body outputs.
    #[must_use]
    pub const fn new(head: Head, body: Body) -> Self {
        Self {
            parts: DocumentParts::new(head, body),
        }
    }

    /// Renders the document through `template`.
    ///
    /// # Errors
    ///
    /// Returns the template engine's rendering error.
    pub fn render(
        self,
        template: &impl DocumentTemplate<Head = Head, Body = Body>,
    ) -> Result<RenderedDocument, ViewError> {
        template.render(self.parts).map(RenderedDocument)
    }
}

#[cfg(feature = "leptos-ssr")]
impl<Head, Body> Document<Head, Body> {
    /// Renders head and body template factories inside the framework's default document.
    ///
    /// The factories are invoked under one request-scoped Leptos owner, so component state,
    /// context, and hydration data are shared across the document and disposed after rendering.
    /// Use [`Self::render_with_leptos`] when an application supplies its own document template.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let page = Document::new(
    ///     || head_template(host.clone()),
    ///     || body_template(host),
    /// )
    /// .render_leptos();
    /// ```
    ///
    /// Call [`initialize_leptos_ssr`] once before rendering, or initialize
    /// [`LeptosExecutor`] with the application's executor. The executor is process-wide.
    ///
    /// # Errors
    ///
    /// Returns an error when metadata injection cannot produce the document chunk.
    /// The application must initialize a Leptos executor before rendering.
    pub fn render_leptos<HeadView, BodyView>(self) -> Result<RenderedDocument, ViewError>
    where
        Head: FnOnce() -> HeadView + Send + 'static,
        Body: FnOnce() -> BodyView + Send + 'static,
        HeadView: IntoView + 'static,
        BodyView: IntoView + 'static,
    {
        self.render_with_leptos(|head, body| {
            view! {
                <!DOCTYPE html>
                <html lang="en">
                    <head>{head()}</head>
                    <body>{body()}</body>
                </html>
            }
        })
    }

    /// Renders through an application-owned complete document template.
    ///
    /// The template receives one-shot head and body content slots. It can call an
    /// AOT-generated Rangular document component with these slots, or compose them
    /// with another Leptos view. The template and both slots are rendered under the
    /// same request-scoped owner; the returned view must contain the complete document.
    /// Call [`initialize_leptos_ssr`] once before rendering, or initialize
    /// [`LeptosExecutor`] with the application's executor. The executor is process-wide.
    ///
    /// # Errors
    ///
    /// Returns an error when metadata injection cannot produce the document chunk.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let page = Document::new(
    ///     || head_template(host.clone()),
    ///     || body_template(host.clone()),
    /// )
    /// .render_with_leptos(|head, body| document_template(host, head, body));
    /// ```
    pub fn render_with_leptos<HeadView, BodyView, View>(
        self,
        template: impl FnOnce(Children, Children) -> View,
    ) -> Result<RenderedDocument, ViewError>
    where
        Head: FnOnce() -> HeadView + Send + 'static,
        Body: FnOnce() -> BodyView + Send + 'static,
        HeadView: IntoView + 'static,
        BodyView: IntoView + 'static,
        View: RenderHtml,
    {
        let (meta_context, meta_output) = ServerMetaContext::new();
        let owner = Owner::new_root(Some(Arc::new(SsrSharedContext::new())));
        let DocumentParts { head, body } = self.parts;
        let html = owner.with(move || {
            provide_context(meta_context);
            let head: Children =
                Box::new(move || view! { <ManagedHead>{head()}</ManagedHead> }.into_any());
            let body: Children = Box::new(move || body().into_any());
            template(head, body).to_html()
        });
        owner.cleanup();
        let html = futures::executor::block_on(async move {
            let input = futures::stream::iter([html]);
            let mut output = Box::pin(meta_output.inject_meta_context(input).await);
            output
                .next()
                .await
                .ok_or_else(|| ViewError::msg("metadata injection produced no document chunk"))
        })?;
        Ok(RenderedDocument(html))
    }
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
