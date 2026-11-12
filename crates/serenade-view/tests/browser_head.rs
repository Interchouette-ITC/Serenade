//! Browser-level head lifecycle and hydration coverage.

#![cfg(all(target_arch = "wasm32", feature = "leptos-client"))]
#![allow(clippy::empty_enums)]

use leptos::{mount::mount_to, prelude::*};
use serenade_view::{ManagedHead, ManagedLink, ManagedMeta, ManagedTitle};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{Document, HtmlElement, Window};

wasm_bindgen_test_configure!(run_in_browser);

fn browser_document() -> Document {
    web_sys::window()
        .expect("browser window")
        .document()
        .expect("browser document")
}

fn attribute(document: &Document, selector: &str, name: &str) -> String {
    document
        .query_selector(selector)
        .expect("valid selector")
        .expect("matching head entry")
        .get_attribute(name)
        .expect("head attribute")
}

fn append_test_root(document: &Document) -> HtmlElement {
    let root = document
        .create_element("div")
        .expect("create test root")
        .dyn_into::<HtmlElement>()
        .expect("test root element");
    document
        .body()
        .expect("document body")
        .append_child(&root)
        .expect("append test root");
    root
}

#[allow(clippy::future_not_send)]
async fn next_browser_task() {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        browser_window()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
            .expect("schedule browser task");
    });
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .expect("wait for browser task");
}

fn browser_window() -> Window {
    web_sys::window().expect("browser window")
}

fn click_button(document: &Document, id: &str) {
    document
        .get_element_by_id(id)
        .unwrap_or_else(|| panic!("missing button {id}"))
        .dyn_into::<HtmlElement>()
        .expect("button element")
        .click();
}

fn assert_head(document: &Document, title: &str, description: &str, canonical: &str) {
    assert_eq!(document.title(), title);
    assert_eq!(
        attribute(document, "head meta[name='description']", "content"),
        description
    );
    assert_eq!(
        attribute(document, "head link[rel='canonical']", "href"),
        canonical
    );
}

#[wasm_bindgen_test]
fn browser_document_is_available() {
    assert!(browser_document().body().is_some());
}

#[component]
fn BrowserHead(route: RwSignal<bool>) -> impl IntoView {
    view! {
        <ManagedHead>
            <ManagedTitle head_key="title" text="Shared title" />
            <ManagedMeta
                head_key="meta:name:description"
                name="description"
                content="Shared description"
            />
            <ManagedLink
                head_key="link:canonical"
                rel="canonical"
                href="/shared"
            />
            <ManagedMeta property="og:image" content="/images/one.jpg" />
            <ManagedMeta property="og:image" content="/images/two.jpg" />
            {move || {
                if route.get() {
                    view! {
                        <ManagedTitle head_key="title" text="Route title" />
                        <ManagedMeta
                            head_key="meta:name:description"
                            name="description"
                            content="Route description"
                        />
                        <ManagedLink
                            head_key="link:canonical"
                            rel="canonical"
                            href="/route"
                        />
                    }
                        .into_any()
                } else {
                    ().into_any()
                }
            }}
        </ManagedHead>
    }
}

#[component]
fn BrowserBody(route: RwSignal<bool>) -> impl IntoView {
    let clicks = RwSignal::new(0);
    view! {
        <main>
            <button id="route" on:click=move |_| route.set(true)>"Route"</button>
            <button id="shared" on:click=move |_| route.set(false)>"Shared"</button>
            <button
                id="counter"
                on:click=move |_| clicks.update(|count| *count += 1)
            >
                {move || clicks.get()}
            </button>
        </main>
    }
}

#[allow(clippy::future_not_send)]
#[wasm_bindgen_test]
async fn head_navigation_replaces_keyed_entries_preserves_repeats_and_cleans_up() {
    let document = browser_document();
    let original_title = document.title();
    let head = document
        .head()
        .expect("document head")
        .unchecked_into::<HtmlElement>();
    let head_marker = document.create_comment("HEAD");
    head.append_child(&head_marker).expect("append head marker");
    let body_root = append_test_root(&document);

    let route = RwSignal::new(false);
    let head_mount = mount_to(head.clone(), move || view! { <BrowserHead route=route /> });
    let body_mount = mount_to(
        body_root.clone(),
        move || view! { <BrowserBody route=route /> },
    );
    next_browser_task().await;

    assert_head(&document, "Shared title", "Shared description", "/shared");
    assert_eq!(
        document
            .query_selector_all("head meta[property='og:image']")
            .expect("valid selector")
            .length(),
        2
    );

    click_button(&document, "route");
    next_browser_task().await;
    assert_head(&document, "Route title", "Route description", "/route");

    click_button(&document, "shared");
    next_browser_task().await;
    assert_head(&document, "Shared title", "Shared description", "/shared");

    click_button(&document, "counter");
    next_browser_task().await;
    assert_eq!(
        document
            .get_element_by_id("counter")
            .expect("counter button")
            .text_content()
            .expect("counter text"),
        "1"
    );

    drop(head_mount);
    drop(body_mount);
    body_root.remove();
    head.remove_child(&head_marker).expect("remove head marker");
    next_browser_task().await;
    assert_eq!(document.title(), original_title);
    assert!(
        document
            .query_selector("head meta[name='description']")
            .expect("valid selector")
            .is_none()
    );
    assert!(
        document
            .query_selector("head link[rel='canonical']")
            .expect("valid selector")
            .is_none()
    );
}

#[wasm_bindgen_test]
fn hydration_attaches_interactivity_to_server_rendered_markup() {
    use leptos::mount::hydrate_from;

    let document = browser_document();
    let original_title = document.title();
    let root = append_test_root(&document);
    root.set_inner_html(include_str!("fixtures/hydration.html"));

    let hydration = hydrate_from(root.clone(), || {
        view! {
            <main>
                <button
                    id="hydrated-action"
                    on:click=move |_| browser_document().set_title("Hydrated click")
                >
                    "Hydrated"
                </button>
            </main>
        }
    });

    document
        .get_element_by_id("hydrated-action")
        .expect("hydrated button")
        .dyn_into::<HtmlElement>()
        .expect("button element")
        .click();

    assert_eq!(document.title(), "Hydrated click");
    drop(hydration);
    root.remove();
    document.set_title(&original_title);
}
