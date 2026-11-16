#[cfg(feature = "hydrate")]
fn main() {
    leptos::mount::hydrate_body(serenade_demo_ssg::App);
    // Only reached when hydration succeeded; lets smoke tests tell hydrated
    // pages apart from the static prerendered HTML.
    if let Some(root) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
    {
        _ = root.set_attribute("data-hydrated", "");
    }
}

#[cfg(not(feature = "hydrate"))]
fn main() {}
