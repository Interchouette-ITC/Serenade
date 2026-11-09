//! `demo-html`: HTML-mode demo (Serenade Form + CSRF, Actix listen).
//!
//! Bootstrap + Quill for UI; Clitorine (`assets/clitorine.js`) wires composer helpers.
//! Run: `make html` or `cargo run -p serenade-demo-html`

mod admin;
mod admin_crud;
mod app;
mod embed;
mod emoji;
mod feed;
mod forms;
mod html;
mod i18n;
mod posts;
mod sanitize;
mod search;
mod state;
mod store;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
