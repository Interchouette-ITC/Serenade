use leptos::prelude::*;

#[component]
pub fn HelloUi() -> impl IntoView {
    let clicks = RwSignal::new(0);

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
                <h1>"Hello from Serenade"</h1>
                <p>"A small shared UI component for the demo-monorepo front apps."</p>
                <button on:click=move |_| clicks.update(|count| *count += 1)>
                    {move || format!("Clicked {} times", clicks.get())}
                </button>
            </section>
        </main>
    }
}
