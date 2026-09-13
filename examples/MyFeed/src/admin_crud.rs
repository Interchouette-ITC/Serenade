//! Dogfood `serenade-admin` on `MyFeed` categories.

use std::collections::HashMap;

use serenade_admin::{
    AdminError, AdminField, AdminRegistry, AdminResource, AdminResourceHandler, AdminRow,
    build_resource_form, register_admin_routes, render_delete_form_html, render_edit_form_html,
    render_list_html, render_new_form_html, render_show_html,
};
use serenade_form::{Form, FormStatus};
use serenade_http::{HttpError, Request, Response, RouteCollection};
use serenade_security::HmacCsrfTokenManager;
use serenade_view::{escape_attr, escape_html, path};

use crate::html::{admin_login_page, document, html_response, redirect};
use crate::i18n::Ui;
use crate::store::FeedStore;

/// Resource name used in route ids (`admin_category_*`).
pub const CATEGORY_RESOURCE: &str = "category";

/// Builds the categories admin resource descriptor.
#[must_use]
pub fn category_resource() -> AdminResource {
    AdminResource::new(CATEGORY_RESOURCE, "/admin/categories")
        .list_fields([AdminField::named("name")])
        .show_fields([AdminField::named("name")])
        .form_fields([AdminField::named("name")])
}

/// Registers list/show/new/edit/delete routes for the category resource.
///
/// # Errors
///
/// Propagates route name collisions.
pub fn register_category_admin_routes(collection: &mut RouteCollection) -> Result<(), HttpError> {
    let mut registry = AdminRegistry::new();
    registry
        .register(category_resource())
        .map_err(|err| HttpError::failed(err.to_string()))?;
    register_admin_routes(collection, &registry)
}

/// Persist hook for category labels (id == name).
pub struct CategoryHandler {
    store: FeedStore,
}

impl CategoryHandler {
    /// Wraps the shared feed store.
    #[must_use]
    pub const fn new(store: FeedStore) -> Self {
        Self { store }
    }
}

impl AdminResourceHandler for CategoryHandler {
    fn list(&self) -> Result<Vec<AdminRow>, AdminError> {
        Ok(self
            .store
            .categories()
            .into_iter()
            .map(|name| AdminRow::new(name.clone()).with("name", name))
            .collect())
    }

    fn get(&self, id: &str) -> Result<Option<AdminRow>, AdminError> {
        if self.store.has_category(id) {
            Ok(Some(AdminRow::new(id).with("name", id)))
        } else {
            Ok(None)
        }
    }

    fn create(&self, data: &HashMap<String, String>) -> Result<AdminRow, AdminError> {
        let name = data.get("name").map_or("", String::as_str);
        self.store
            .add_category(name)
            .map_err(|message| AdminError::Persist {
                message: message.to_owned(),
            })?;
        let name = name.trim();
        let name: String = name.chars().take(40).collect();
        Ok(AdminRow::new(name.clone()).with("name", name))
    }

    fn update(&self, id: &str, data: &HashMap<String, String>) -> Result<AdminRow, AdminError> {
        if !self.store.has_category(id) {
            return Err(AdminError::NotFound { id: id.to_owned() });
        }
        let name = data.get("name").map_or("", String::as_str).trim();
        if name.is_empty() {
            return Err(AdminError::Persist {
                message: "Category name is required.".to_owned(),
            });
        }
        if name != id {
            self.store.remove_category(id);
            self.store
                .add_category(name)
                .map_err(|message| AdminError::Persist {
                    message: message.to_owned(),
                })?;
        }
        let name: String = name.chars().take(40).collect();
        Ok(AdminRow::new(name.clone()).with("name", name))
    }

    fn delete(&self, id: &str) -> Result<bool, AdminError> {
        Ok(self.store.remove_category(id))
    }
}

/// Compact list + link for the main `/admin` page.
pub fn categories_panel_html(
    store: &FeedStore,
    routes: &RouteCollection,
) -> Result<String, HttpError> {
    let resource = category_resource();
    let handler = CategoryHandler::new(store.clone());
    let rows = handler
        .list()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let list = render_list_html(&resource, &rows);
    let new_href = path(routes, &resource.new_route_name(), &[])?;
    Ok(format!(
        r#"{list}
<p class="mt-3 mb-0">
  <a class="btn btn-primary btn-sm" href="{new_href}">New category</a>
  <a class="btn btn-outline-secondary btn-sm" href="{list_href}">Open CRUD list</a>
</p>"#,
        list_href = escape_attr(&path(routes, &resource.list_route_name(), &[])?),
        new_href = escape_attr(&new_href),
    ))
}

/// Inputs for category admin HTTP dispatch.
pub struct CategoryAdminCtx<'a> {
    /// Shared feed store.
    pub store: &'a FeedStore,
    /// CSRF manager.
    pub csrf: &'a HmacCsrfTokenManager,
    /// App routes (for reverse URLs).
    pub routes: &'a RouteCollection,
    /// Current request.
    pub request: &'a Request,
    /// Localized UI strings.
    pub ui: &'a Ui<'a>,
    /// Whether the caller is authenticated as admin.
    pub is_admin: bool,
}

/// Dispatches `admin_category_*` routes. Returns `None` when the route is unrelated.
pub fn try_handle(
    route: &str,
    ctx: &CategoryAdminCtx<'_>,
    build_login: impl FnOnce(&HmacCsrfTokenManager, &RouteCollection) -> Result<String, HttpError>,
) -> Result<Option<Response>, HttpError> {
    let resource = category_resource();
    let names = [
        resource.list_route_name(),
        resource.show_route_name(),
        resource.new_route_name(),
        resource.create_route_name(),
        resource.edit_route_name(),
        resource.update_route_name(),
        resource.delete_route_name(),
    ];
    if !names.iter().any(|name| name == route) {
        return Ok(None);
    }
    if !ctx.is_admin {
        return Ok(Some(login_denied(
            ctx.ui,
            ctx.routes,
            ctx.csrf,
            build_login,
        )?));
    }
    let handler = CategoryHandler::new(ctx.store.clone());
    match route {
        name if name == resource.list_route_name() => {
            Ok(Some(handle_list(&handler, &resource, ctx.routes, ctx.ui)?))
        }
        name if name == resource.show_route_name() => Ok(Some(handle_show(
            &handler,
            &resource,
            ctx.request,
            ctx.routes,
            ctx.ui,
        )?)),
        name if name == resource.new_route_name() => {
            Ok(Some(handle_new(&resource, ctx.csrf, ctx.routes, ctx.ui)?))
        }
        name if name == resource.create_route_name() => Ok(Some(handle_create(
            &handler,
            &resource,
            ctx.csrf,
            ctx.request,
            ctx.routes,
            ctx.ui,
        )?)),
        name if name == resource.edit_route_name() => Ok(Some(handle_edit(
            &handler,
            &resource,
            ctx.csrf,
            ctx.request,
            ctx.routes,
            ctx.ui,
        )?)),
        name if name == resource.update_route_name() => Ok(Some(handle_update(
            &handler,
            &resource,
            ctx.csrf,
            ctx.request,
            ctx.routes,
            ctx.ui,
        )?)),
        name if name == resource.delete_route_name() => Ok(Some(handle_delete(
            &handler,
            &resource,
            ctx.csrf,
            ctx.request,
        )?)),
        _ => Ok(None),
    }
}

fn handle_list(
    handler: &CategoryHandler,
    resource: &AdminResource,
    routes: &RouteCollection,
    ui: &Ui<'_>,
) -> Result<Response, HttpError> {
    let rows = handler
        .list()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let body = render_list_html(resource, &rows);
    let new_href = path(routes, &resource.new_route_name(), &[])?;
    Ok(crud_page(
        ui,
        routes,
        &format!(
            r#"{body}<p><a class="btn btn-primary btn-sm" href="{href}">New</a></p>"#,
            href = escape_attr(&new_href)
        ),
    ))
}

fn handle_show(
    handler: &CategoryHandler,
    resource: &AdminResource,
    request: &Request,
    routes: &RouteCollection,
    ui: &Ui<'_>,
) -> Result<Response, HttpError> {
    let id = path_id(request)?;
    let Some(row) = handler
        .get(&id)
        .map_err(|err| HttpError::failed(err.to_string()))?
    else {
        return Err(HttpError::not_found("category"));
    };
    let show = render_show_html(resource, &row);
    let edit = path(routes, &resource.edit_route_name(), &[("id", id.as_str())])?;
    let list = path(routes, &resource.list_route_name(), &[])?;
    Ok(crud_page(
        ui,
        routes,
        &format!(
            r#"{show}
<p class="d-flex gap-2">
  <a class="btn btn-outline-primary btn-sm" href="{edit}">Edit</a>
  <a class="btn btn-outline-secondary btn-sm" href="{list}">Back</a>
</p>"#,
            edit = escape_attr(&edit),
            list = escape_attr(&list),
        ),
    ))
}

fn handle_new(
    resource: &AdminResource,
    csrf: &HmacCsrfTokenManager,
    routes: &RouteCollection,
    ui: &Ui<'_>,
) -> Result<Response, HttpError> {
    let html =
        render_new_form_html(resource, csrf).map_err(|err| HttpError::failed(err.to_string()))?;
    Ok(crud_page(ui, routes, &html))
}

fn handle_create(
    handler: &CategoryHandler,
    resource: &AdminResource,
    csrf: &HmacCsrfTokenManager,
    request: &Request,
    routes: &RouteCollection,
    ui: &Ui<'_>,
) -> Result<Response, HttpError> {
    let mut form = build_resource_form(resource, resource.new_path());
    match form.handle_request(request, csrf) {
        Ok(FormStatus::Bound) => match handler.create(&form.data()) {
            Ok(row) => Ok(redirect(&resource.show_path_for(row.id()))),
            Err(err) => Ok(crud_page(
                ui,
                routes,
                &format!(
                    r#"<div class="alert alert-danger">{msg}</div>{form}"#,
                    msg = escape_html(&err.to_string()),
                    form = render_new_form_html(resource, csrf)
                        .map_err(|e| HttpError::failed(e.to_string()))?
                ),
            )),
        },
        _ => Ok(redirect(&resource.new_path())),
    }
}

fn handle_edit(
    handler: &CategoryHandler,
    resource: &AdminResource,
    csrf: &HmacCsrfTokenManager,
    request: &Request,
    routes: &RouteCollection,
    ui: &Ui<'_>,
) -> Result<Response, HttpError> {
    let id = path_id(request)?;
    let Some(row) = handler
        .get(&id)
        .map_err(|err| HttpError::failed(err.to_string()))?
    else {
        return Err(HttpError::not_found("category"));
    };
    let edit = render_edit_form_html(resource, &row, csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let delete = render_delete_form_html(resource, &id, csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    Ok(crud_page(ui, routes, &format!("{edit}\n{delete}")))
}

fn handle_update(
    handler: &CategoryHandler,
    resource: &AdminResource,
    csrf: &HmacCsrfTokenManager,
    request: &Request,
    routes: &RouteCollection,
    ui: &Ui<'_>,
) -> Result<Response, HttpError> {
    let id = path_id(request)?;
    let mut form = build_resource_form(resource, resource.edit_path_for(&id));
    match form.handle_request(request, csrf) {
        Ok(FormStatus::Bound) => match handler.update(&id, &form.data()) {
            Ok(row) => Ok(redirect(&resource.show_path_for(row.id()))),
            Err(err) => {
                let Some(row) = handler
                    .get(&id)
                    .map_err(|e| HttpError::failed(e.to_string()))?
                else {
                    return Err(HttpError::not_found("category"));
                };
                let edit = render_edit_form_html(resource, &row, csrf)
                    .map_err(|e| HttpError::failed(e.to_string()))?;
                Ok(crud_page(
                    ui,
                    routes,
                    &format!(
                        r#"<div class="alert alert-danger">{msg}</div>{edit}"#,
                        msg = escape_html(&err.to_string()),
                    ),
                ))
            }
        },
        _ => Ok(redirect(&resource.edit_path_for(&id))),
    }
}

fn handle_delete(
    handler: &CategoryHandler,
    resource: &AdminResource,
    csrf: &HmacCsrfTokenManager,
    request: &Request,
) -> Result<Response, HttpError> {
    let id = path_id(request)?;
    let mut form = Form::builder(format!("admin_{}_delete", resource.name()))
        .action(resource.delete_path_for(&id))
        .build();
    if form.handle_request(request, csrf).is_err() {
        return Ok(redirect(&resource.edit_path_for(&id)));
    }
    let _ = handler
        .delete(&id)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    Ok(redirect(&resource.list_path()))
}

fn path_id(request: &Request) -> Result<String, HttpError> {
    request
        .attributes()
        .get::<String>("id")
        .cloned()
        .ok_or_else(|| HttpError::bad_request("missing id"))
}

fn crud_page(ui: &Ui<'_>, routes: &RouteCollection, inner: &str) -> Response {
    let feed = path(routes, "feed", &[]).unwrap_or_else(|_| "/".into());
    let admin = path(routes, "admin", &[]).unwrap_or_else(|_| "/admin".into());
    let body = format!(
        r#"
<div class="myfeed-shell">
  <header class="d-flex flex-wrap gap-3 mb-4 pb-3 border-bottom">
    <div class="me-auto">
      <h1 class="myfeed-brand">{title}</h1>
      <p class="myfeed-tag">serenade-admin categories</p>
    </div>
    <nav class="myfeed-nav d-flex align-items-center gap-3 pt-2">
      <a class="link-secondary" href="{feed}">{nav_feed}</a>
      <a class="admin-link btn btn-outline-primary btn-sm" href="{admin}">{nav_admin}</a>
    </nav>
  </header>
  <main>{inner}</main>
</div>
"#,
        title = escape_html(&ui.t("categories_title")),
        feed = escape_attr(&feed),
        admin = escape_attr(&admin),
        nav_feed = escape_html(&ui.t("nav_feed")),
        nav_admin = escape_html(&ui.t("nav_admin")),
    );
    html_response(
        200,
        document(
            &ui.t("categories_title"),
            &body,
            false,
            ui.locale().language(),
        ),
    )
}

/// Login denial helper shared with main admin handlers.
pub fn login_denied(
    ui: &Ui<'_>,
    routes: &RouteCollection,
    csrf: &HmacCsrfTokenManager,
    build_login: impl FnOnce(&HmacCsrfTokenManager, &RouteCollection) -> Result<String, HttpError>,
) -> Result<Response, HttpError> {
    let login = build_login(csrf, routes)?;
    Ok(html_response(
        401,
        admin_login_page(ui, routes, &login, Some("Sign in first.")),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn category_handler_crud_roundtrip() {
        let store = FeedStore::open_memory();
        let handler = CategoryHandler::new(store);
        let created = handler
            .create(&HashMap::from([("name".to_owned(), "Dogs".to_owned())]))
            .expect("create");
        assert_eq!(created.id(), "Dogs");
        assert!(handler.get("Dogs").expect("get").is_some());
        let updated = handler
            .update(
                "Dogs",
                &HashMap::from([("name".to_owned(), "Cats".to_owned())]),
            )
            .expect("update");
        assert_eq!(updated.id(), "Cats");
        assert!(handler.delete("Cats").expect("delete"));
        assert!(handler.get("Cats").expect("missing").is_none());
    }

    #[test]
    fn category_resource_route_names() {
        let resource = category_resource();
        assert_eq!(resource.list_route_name(), "admin_category_list");
        assert_eq!(resource.new_path(), "/admin/categories/new");
    }
}
