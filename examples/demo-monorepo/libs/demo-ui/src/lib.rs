use leptos::prelude::*;

#[component]
pub fn HelloUi(#[prop(default = "home")] page: &'static str) -> impl IntoView {
    let clicks = RwSignal::new(0);
    let (title, description) = match page {
        "about" => (
            "About Serenade",
            "Serenade builds web apps with shared, framework-rendered UI.",
        ),
        _ => (
            "Hello from Serenade",
            "A shared hello UI component rendered by Serenade's front apps.",
        ),
    };

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
                <h1>{title}</h1>
                <p>{description}</p>
                <button on:click=move |_| clicks.update(|count| *count += 1)>
                    {move || format!("Clicked {} times", clicks.get())}
                </button>
            </section>
        </main>
    }
}
