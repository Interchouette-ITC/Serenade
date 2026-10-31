use leptos::prelude::*;

#[cfg(feature = "hydrate")]
#[component]
pub fn App() -> impl IntoView {
    let path = web_sys::window()
        .expect("browser window is available during hydration")
        .location()
        .pathname()
        .expect("browser pathname is available during hydration");
    let page = if path == "/about" || path == "/about/" {
        "about"
    } else {
        "home"
    };

    view! { <serenade_demo_ui::HelloUi page=page /> }
}

#[cfg(feature = "ssr")]
pub fn render_page(page: &'static str) -> String {
    let owner = Owner::new_root(Some(std::sync::Arc::new(
        hydration_context::SsrSharedContext::new(),
    )));
    owner.with(|| view! { <serenade_demo_ui::HelloUi page=page /> }.to_html())
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::render_page;

    #[test]
    fn prerendered_pages_include_route_content_and_interactive_state() {
        let home = render_page("home");
        let about = render_page("about");

        assert!(home.contains("<h1>Hello from Serenade</h1>"));
        assert!(home.contains("Clicked 0 times"));
        assert!(about.contains("<h1>About Serenade</h1>"));
        assert!(about.contains("Clicked 0 times"));
    }
}
