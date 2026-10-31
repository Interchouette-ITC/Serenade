use leptos::prelude::*;
use wasm_bindgen::JsCast;

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

fn main() {
    mount_to_body(App);
}
