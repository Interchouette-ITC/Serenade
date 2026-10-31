#[cfg(feature = "hydrate")]
fn main() {
    leptos::mount::hydrate_body(serenade_demo_ssg::App);
}

#[cfg(not(feature = "hydrate"))]
fn main() {}
