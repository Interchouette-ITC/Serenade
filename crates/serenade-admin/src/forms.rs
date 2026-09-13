//! Build create / edit / delete forms from resource field maps.

use serenade_form::{Form, FormError};
use serenade_http::Method;
use serenade_security::CsrfTokenManager;

use crate::{AdminResource, AdminRow};

/// Builds a CSRF-enabled create/edit form from [`AdminResource::form`] fields.
#[must_use]
pub fn build_resource_form(resource: &AdminResource, action: impl Into<String>) -> Form {
    let mut builder = Form::builder(format!("admin_{}_form", resource.name()))
        .method(Method::Post)
        .action(action)
        .csrf(true);
    for field in resource.form() {
        builder = builder.field(field.name(), Vec::new());
    }
    builder.build()
}

/// Prefills an edit form from an [`AdminRow`].
pub fn prefill_resource_form(form: &mut Form, row: &AdminRow) {
    for (name, value) in row.values() {
        let _ = form.set(name, value);
    }
}

/// Builds a CSRF-only delete confirmation form (POST).
#[must_use]
pub fn build_delete_form(resource: &AdminResource, action: impl Into<String>) -> Form {
    Form::builder(format!("admin_{}_delete", resource.name()))
        .method(Method::Post)
        .action(action)
        .csrf(true)
        .build()
}

/// Issues CSRF and returns escaped form HTML.
///
/// # Errors
///
/// Returns [`FormError`] when CSRF or render fails.
pub fn render_prepared_form(
    form: &mut Form,
    manager: &dyn CsrfTokenManager,
) -> Result<String, FormError> {
    form.prepare_csrf(manager)?;
    Ok(form.render()?.as_html().to_owned())
}

/// Convenience: new form HTML for `resource`.
///
/// # Errors
///
/// Returns [`FormError`] when CSRF or render fails.
pub fn render_new_form_html(
    resource: &AdminResource,
    manager: &dyn CsrfTokenManager,
) -> Result<String, FormError> {
    let mut form = build_resource_form(resource, resource.new_path());
    let body = render_prepared_form(&mut form, manager)?;
    Ok(wrap_form_section(resource, "new", &body))
}

/// Convenience: edit form HTML prefilled from `row`.
///
/// # Errors
///
/// Returns [`FormError`] when CSRF or render fails.
pub fn render_edit_form_html(
    resource: &AdminResource,
    row: &AdminRow,
    manager: &dyn CsrfTokenManager,
) -> Result<String, FormError> {
    let mut form = build_resource_form(resource, resource.edit_path_for(row.id()));
    prefill_resource_form(&mut form, row);
    let body = render_prepared_form(&mut form, manager)?;
    Ok(wrap_form_section(resource, "edit", &body))
}

/// Convenience: delete form HTML for `id`.
///
/// # Errors
///
/// Returns [`FormError`] when CSRF or render fails.
pub fn render_delete_form_html(
    resource: &AdminResource,
    id: &str,
    manager: &dyn CsrfTokenManager,
) -> Result<String, FormError> {
    let mut form = build_delete_form(resource, resource.delete_path_for(id));
    let body = render_prepared_form(&mut form, manager)?;
    Ok(wrap_form_section(resource, "delete", &body))
}

fn wrap_form_section(resource: &AdminResource, kind: &str, body: &str) -> String {
    format!(
        "<section class=\"serenade-admin-{kind}\">\n<h1>{name} {kind}</h1>\n{body}\n</section>\n",
        name = serenade_form::escape_html(resource.name()),
    )
}
