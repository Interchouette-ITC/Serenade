//! Optional EasyAdmin-shaped Admin CRUD helpers.
//!
//! Opt in with [`AdminBundle`] / [`AdminExtension`]. Not registered by
//! `FrameworkBundle`. Apps implement [`AdminResourceHandler`] for persistence.

mod bundle;
mod error;
mod field;
mod forms;
mod handler;
mod html;
mod registry;
mod resource;
mod routes;
mod row;

pub use bundle::{ADMIN_BUNDLE, ADMIN_REGISTRY_SERVICE, AdminBundle, AdminExtension};
pub use error::AdminError;
pub use field::AdminField;
pub use forms::{
    build_delete_form, build_resource_form, prefill_resource_form, render_delete_form_html,
    render_edit_form_html, render_new_form_html, render_prepared_form,
};
pub use handler::AdminResourceHandler;
pub use html::{render_list_html, render_show_html};
pub use registry::AdminRegistry;
pub use resource::AdminResource;
pub use routes::register_admin_routes;
pub use row::AdminRow;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
