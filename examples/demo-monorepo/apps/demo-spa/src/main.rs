use leptos::mount::mount_to_body;
use leptos::prelude::*;

fn main() {
    mount_to_body(|| view! { <serenade_demo_ui::HelloUi /> });
}
