use leptos::prelude::*;
use rangular_aot::HostCell;
use serenade_view::ManagedHead;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{Element, Event, HtmlElement, Window};

#[path = "../../../libs/cms_hello_host.rs"]
mod cms_hello_host;

#[allow(dead_code)]
mod cms_hello {
    use leptos::prelude::*;

    include!(concat!(env!("OUT_DIR"), "/cms_hello.rs"));
}

use cms_hello_host::CmsHelloHost;

fn cms_head_view(host: HostCell<CmsHelloHost>) -> impl IntoView {
    view! { <ManagedHead>{cms_hello::cms_head(host)}</ManagedHead> }
}

fn clear_server_rendered_head(document: &web_sys::Document) {
    // Leptos claims the existing title node; only replace explicitly owned metadata.
    let nodes = document
        .query_selector_all("head > [data-serenade-head='ssr']")
        .expect("valid server-owned head selector");
    for index in 0..nodes.length() {
        if let Some(node) = nodes.item(index) {
            node.dyn_into::<Element>()
                .expect("server-rendered head entry is an element")
                .remove();
        }
    }
}

#[cfg(test)]
fn snapshot_children(parent: &HtmlElement) -> Vec<web_sys::Node> {
    let mut children = Vec::new();
    let mut current = parent.first_child();
    while let Some(node) = current {
        current = node.next_sibling();
        children.push(node);
    }
    children
}

#[cfg(test)]
fn remove_added_nodes(parent: &HtmlElement, original: &[web_sys::Node]) {
    let mut current = parent.first_child();
    while let Some(node) = current {
        current = node.next_sibling();
        if !original
            .iter()
            .any(|original| node.is_same_node(Some(original)))
        {
            parent
                .remove_child(&node)
                .expect("remove added browser test node");
        }
    }
}

fn create_cms_host(
    window: &Window,
    route_specific: bool,
) -> (HostCell<CmsHelloHost>, Closure<dyn FnMut(Event)>) {
    let navigation_window = window.clone();
    let host = CmsHelloHost::with_navigator(route_specific, move |route_specific| {
        let path = if route_specific {
            "/cms/hello"
        } else {
            "/cms/hello?view=shared"
        };
        navigation_window
            .history()
            .expect("browser history is available")
            .push_state_with_url(&JsValue::NULL, "", Some(path))
            .expect("update browser history");
    });
    let route = host.route_signal();
    let popstate_window = window.clone();
    let popstate = Closure::wrap(Box::new(move |_event: Event| {
        let search = popstate_window
            .location()
            .search()
            .expect("browser search is available");
        route.set(cms_hello_host::route_specific_from_query(Some(&search)));
    }) as Box<dyn FnMut(Event)>);
    window
        .add_event_listener_with_callback("popstate", popstate.as_ref().unchecked_ref())
        .expect("install browser history listener");

    (HostCell::new(host), popstate)
}

#[component]
fn App() -> impl IntoView {
    let message = RwSignal::new(String::from("Loading from demo-back…"));
    let clicks = RwSignal::new(0);

    wasm_bindgen_futures::spawn_local(async move {
        let result = async {
            let window = web_sys::window().ok_or("browser window unavailable")?;
            let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str("/hello"))
                .await
                .map_err(|_| "request to /hello failed")?
                .dyn_into::<web_sys::Response>()
                .map_err(|_| "unexpected response from /hello")?;
            let text = wasm_bindgen_futures::JsFuture::from(
                response
                    .text()
                    .map_err(|_| "could not read /hello response")?,
            )
            .await
            .map_err(|_| "could not read /hello response")?
            .as_string()
            .ok_or("invalid text response from /hello")?;
            let response: serenade_demo_contract::HelloResponse =
                serde_json::from_str(&text).map_err(|_| "invalid JSON from /hello")?;
            Ok::<_, &'static str>(response.message)
        }
        .await;
        message.set(result.unwrap_or_else(str::to_owned));
    });

    view! {
        <main class="hello">
            <header class="hello__header">
                <img
                    class="hello__banner"
                    src="/serenade-banner.png"
                    alt="Serenade"
                    width="534"
                    height="147"
                />
            </header>
            <section class="hello__content">
                <h1>"Serenade Universal"</h1>
                <p>{move || message.get()}</p>
                <button on:click=move |_| clicks.update(|count| *count += 1)>
                    {move || format!("Clicked {} times", clicks.get())}
                </button>
                <a href="/cms/hello">"Open live server-rendered page"</a>
            </section>
        </main>
    }
}

#[cfg(not(test))]
fn main() {
    let window = web_sys::window().expect("browser window is available");
    let location = window.location();
    let path = location.pathname().expect("browser pathname is available");
    if path == "/cms/hello" {
        let search = location.search().expect("browser search is available");
        let route_specific = cms_hello_host::route_specific_from_query(Some(&search));
        let document = window.document().expect("browser document is available");
        let head = document
            .head()
            .expect("document head is available")
            .unchecked_into::<HtmlElement>();
        let (host, popstate) = create_cms_host(&window, route_specific);
        popstate.forget();

        clear_server_rendered_head(&document);
        let head_host = host.clone();
        leptos::mount::mount_to(head, move || cms_head_view(head_host)).forget();
        leptos::mount::hydrate_body(move || cms_hello::cms_body(host));
    } else {
        leptos::mount::mount_to_body(App);
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod browser_tests {
    use leptos::mount::{hydrate_from, mount_to};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_sys::{Document, js_sys::Promise};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn attribute(document: &Document, selector: &str, name: &str) -> String {
        document
            .query_selector(selector)
            .expect("valid selector")
            .expect("matching head entry")
            .get_attribute(name)
            .expect("head attribute")
    }

    fn click(document: &Document, id: &str) {
        document
            .get_element_by_id(id)
            .unwrap_or_else(|| panic!("missing button {id}"))
            .dyn_into::<HtmlElement>()
            .expect("button element")
            .click();
    }

    #[allow(clippy::future_not_send)]
    async fn next_browser_task() {
        let promise = Promise::new(&mut |resolve, _| {
            web_sys::window()
                .expect("browser window")
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
                .expect("schedule browser task");
        });
        JsFuture::from(promise)
            .await
            .expect("wait for browser task");
    }

    fn assert_head(document: &Document, title: &str, description: &str, canonical: &str) {
        assert_eq!(document.title(), title);
        assert_eq!(
            attribute(
                document,
                "head meta[name='description']:not(#unrelated-description)",
                "content"
            ),
            description
        );
        assert_eq!(
            attribute(
                document,
                "head link[rel='canonical']:not(#unrelated-canonical)",
                "href"
            ),
            canonical
        );
        assert_eq!(
            document
                .query_selector_all("head meta[property='og:image']:not(#unrelated-social)")
                .expect("valid selector")
                .length(),
            2
        );
    }

    #[allow(clippy::future_not_send)]
    #[wasm_bindgen_test]
    async fn cms_ssr_markup_hydrates_and_navigates_without_replacing_the_document() {
        let window = web_sys::window().expect("browser window");
        let document = window.document().expect("browser document");
        let document_element = document.document_element().expect("document element");
        let original_url = window.location().href().expect("browser URL");
        let original_title = document.title();
        let head = document
            .head()
            .expect("document head")
            .unchecked_into::<HtmlElement>();
        let body = document
            .body()
            .expect("document body")
            .unchecked_into::<HtmlElement>();
        let original_head_nodes = snapshot_children(&head);
        let original_body_nodes = snapshot_children(&body);
        head.insert_adjacent_html("afterbegin", include_str!("fixtures/cms-hello-head.html"))
            .expect("insert server-rendered head fixture");
        head.insert_adjacent_html(
            "beforeend",
            r#"
            <meta id="unrelated-charset" charset="utf-8">
            <meta id="unrelated-viewport" name="viewport" content="width=device-width">
            <meta id="unrelated-description" name="description" content="Unrelated">
            <meta id="unrelated-social" property="og:image" content="/other.png">
            <link id="unrelated-canonical" rel="canonical" href="/other">
            <link id="unrelated-style" rel="stylesheet" href="/style.css">
            <link id="unrelated-icon" rel="icon" type="image/x-icon" href="/favicon.ico">
            <script id="unrelated-json" type="application/ld+json">{"name":"unrelated"}</script>
        "#,
        )
        .expect("insert unrelated application head entries");
        let unrelated_nodes: Vec<_> = [
            "unrelated-charset",
            "unrelated-viewport",
            "unrelated-description",
            "unrelated-social",
            "unrelated-canonical",
            "unrelated-style",
            "unrelated-icon",
            "unrelated-json",
        ]
        .into_iter()
        .map(|id| document.get_element_by_id(id).expect("unrelated node"))
        .collect();
        let server_title = document
            .query_selector("head title")
            .expect("title selector")
            .expect("server title");
        body.insert_adjacent_html("afterbegin", include_str!("fixtures/cms-hello-body.html"))
            .expect("insert server-rendered body fixture");

        let bootstrap_element = document
            .query_selector("script[data-serenade-universal-bootstrap]")
            .expect("valid bootstrap selector")
            .expect("CMS bootstrap script is in the fixture");
        assert_eq!(
            bootstrap_element.get_attribute("data-wasm-js").as_deref(),
            Some("/serenade-demo-front.js")
        );
        let heading = document
            .get_element_by_id("cms-heading")
            .expect("server-rendered heading");
        let main = document
            .query_selector("body > main")
            .expect("valid main selector")
            .expect("server-rendered main");
        let (host, popstate) = create_cms_host(&window, true);
        clear_server_rendered_head(&document);
        for node in &unrelated_nodes {
            assert!(
                node.parent_node()
                    .expect("unrelated head node retained")
                    .is_same_node(Some(head.as_ref()))
            );
        }
        let head_host = host.clone();
        let head_mount = mount_to(head.clone(), move || cms_head_view(head_host));
        let body_hydration = hydrate_from(body.clone(), move || cms_hello::cms_body(host));
        next_browser_task().await;
        assert!(
            server_title.is_same_node(
                document
                    .query_selector("head title")
                    .expect("title selector")
                    .as_ref()
                    .map(|node| node.as_ref())
            )
        );
        for node in &unrelated_nodes {
            assert!(
                node.parent_node()
                    .expect("unrelated head node retained after mount")
                    .is_same_node(Some(head.as_ref()))
            );
        }

        assert!(
            document_element.is_same_node(
                document
                    .document_element()
                    .as_ref()
                    .map(|element| element.unchecked_ref())
            )
        );
        assert!(
            heading.is_same_node(
                document
                    .get_element_by_id("cms-heading")
                    .as_ref()
                    .map(|element| element.unchecked_ref())
            ),
            "hydration keeps the server-rendered heading node"
        );
        assert!(
            main.is_same_node(
                document
                    .query_selector("body > main")
                    .expect("valid main selector")
                    .as_ref()
                    .map(|element| element.unchecked_ref())
            ),
            "hydration keeps the server-rendered body node"
        );
        assert_head(
            &document,
            "Serenade CMS & <Hello>",
            "A request-time generated description with & <angle brackets>.",
            "/cms/hello?source=universal&mode=ssr",
        );
        assert_eq!(
            document
                .get_element_by_id("cms-heading")
                .expect("hydrated heading")
                .text_content()
                .as_deref(),
            Some("Hello from Rangular")
        );

        click(&document, "cms-counter");
        next_browser_task().await;
        assert_eq!(
            document
                .get_element_by_id("cms-counter")
                .expect("counter button")
                .text_content()
                .as_deref(),
            Some("Clicked 1 times")
        );

        click(&document, "show-shared");
        next_browser_task().await;
        assert_head(
            &document,
            "Serenade Universal",
            "A shared description for the Universal document.",
            "/",
        );
        assert_eq!(
            document
                .get_element_by_id("cms-heading")
                .expect("shared heading")
                .text_content()
                .as_deref(),
            Some("The shared Universal route")
        );
        assert_eq!(window.location().search().as_deref(), Ok("?view=shared"));

        click(&document, "cms-counter");
        next_browser_task().await;
        assert_eq!(
            document
                .get_element_by_id("cms-counter")
                .expect("shared counter button")
                .text_content()
                .as_deref(),
            Some("Clicked 2 times")
        );

        click(&document, "show-route");
        next_browser_task().await;
        assert_head(
            &document,
            "Serenade CMS & <Hello>",
            "A request-time generated description with & <angle brackets>.",
            "/cms/hello?source=universal&mode=ssr",
        );

        for node in &unrelated_nodes {
            assert!(
                node.parent_node()
                    .expect("unrelated node retained after navigation")
                    .is_same_node(Some(head.as_ref()))
            );
        }
        window
            .history()
            .expect("browser history")
            .replace_state_with_url(&JsValue::NULL, "", Some(&original_url))
            .expect("restore browser URL");
        window
            .remove_event_listener_with_callback("popstate", popstate.as_ref().unchecked_ref())
            .expect("remove history listener");
        drop(body_hydration);
        drop(head_mount);
        remove_added_nodes(&head, &original_head_nodes);
        remove_added_nodes(&body, &original_body_nodes);
        document.set_title(&original_title);
        assert_eq!(document.title(), original_title);
    }
}
