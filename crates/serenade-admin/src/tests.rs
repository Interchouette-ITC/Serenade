use std::collections::HashMap;
use std::sync::Mutex;

use serenade_bundle::{
    BundleInterface, Extension, FRAMEWORK_BUNDLE, FrameworkExtension, build_container,
};
use serenade_form::{CSRF_FIELD_NAME, FormStatus};
use serenade_http::{Method, Request, Route, RouteCollection};
use serenade_security::HmacCsrfTokenManager;

use super::*;

#[test]
fn version_is_nonempty() {
    assert_ne!(version(), "");
}

#[test]
fn registry_rejects_duplicate_names() {
    let mut registry = AdminRegistry::new();
    registry
        .register(AdminResource::new("product", "/admin/products"))
        .expect("first");
    let err = registry
        .register(AdminResource::new("product", "/admin/other"))
        .expect_err("dup");
    assert!(matches!(
        err,
        AdminError::DuplicateResource { name } if name == "product"
    ));
    assert!(registry.get("missing").is_none());
    assert_eq!(registry.get("product").expect("get").name(), "product");
    assert_eq!(registry.resources().len(), 1);
}

#[test]
fn resource_paths_and_form_fields() {
    let with_slash = AdminResource::new("product", "/admin/products/")
        .list_fields([AdminField::named("name")])
        .show_fields([AdminField::new("sku", "SKU")])
        .form_fields([AdminField::named("name"), AdminField::named("price")]);
    assert_eq!(with_slash.path_prefix(), "/admin/products/");
    assert_eq!(with_slash.list().len(), 1);
    assert_eq!(with_slash.show().len(), 1);
    assert_eq!(with_slash.form().len(), 2);
    assert_eq!(with_slash.show()[0].name(), "sku");
    assert_eq!(with_slash.list_path(), "/admin/products");
    assert_eq!(with_slash.show_path(), "/admin/products/{id}");
    assert_eq!(with_slash.new_path(), "/admin/products/new");
    assert_eq!(with_slash.edit_path(), "/admin/products/{id}/edit");
    assert_eq!(with_slash.delete_path(), "/admin/products/{id}/delete");
    assert_eq!(with_slash.edit_path_for("9"), "/admin/products/9/edit");
    assert_eq!(with_slash.delete_path_for("9"), "/admin/products/9/delete");
    assert_eq!(with_slash.show_path_for("9"), "/admin/products/9");
    assert_eq!(with_slash.list_route_name(), "admin_product_list");
    assert_eq!(with_slash.show_route_name(), "admin_product_show");
    assert_eq!(with_slash.new_route_name(), "admin_product_new");
    assert_eq!(with_slash.create_route_name(), "admin_product_create");
    assert_eq!(with_slash.edit_route_name(), "admin_product_edit");
    assert_eq!(with_slash.update_route_name(), "admin_product_update");
    assert_eq!(with_slash.delete_route_name(), "admin_product_delete");

    let fallback =
        AdminResource::new("cat", "admin/categories").list_fields([AdminField::named("title")]);
    assert_eq!(fallback.list_path(), "/admin/categories");
    assert_eq!(fallback.show().len(), 1);
    assert_eq!(fallback.form().len(), 1);
    assert_eq!(fallback.show()[0].name(), "title");

    let root = AdminResource::new("root", "/");
    assert_eq!(root.list_path(), "/");
    assert_eq!(root.show_path(), "/{id}");
    assert_eq!(root.new_path(), "/new");
    assert_eq!(root.edit_path_for("1"), "/1/edit");

    let empty_prefix = AdminResource::new("empty", "");
    assert_eq!(empty_prefix.list_path(), "/");
}

#[test]
fn register_routes_crud() {
    let mut registry = AdminRegistry::new();
    registry
        .register(
            AdminResource::new("product", "/admin/products")
                .list_fields([AdminField::named("name"), AdminField::new("price", "Price")]),
        )
        .expect("register");
    let mut collection = RouteCollection::new();
    register_admin_routes(&mut collection, &registry).expect("routes");
    assert_eq!(
        collection.get("admin_product_list").expect("list").path(),
        "/admin/products"
    );
    assert_eq!(
        collection.get("admin_product_new").expect("new").path(),
        "/admin/products/new"
    );
    assert_eq!(
        collection
            .get("admin_product_create")
            .expect("create")
            .path(),
        "/admin/products/new"
    );
    assert_eq!(
        collection.get("admin_product_show").expect("show").path(),
        "/admin/products/{id}"
    );
    assert_eq!(
        collection.get("admin_product_edit").expect("edit").path(),
        "/admin/products/{id}/edit"
    );
    assert_eq!(
        collection
            .get("admin_product_update")
            .expect("update")
            .path(),
        "/admin/products/{id}/edit"
    );
    assert_eq!(
        collection
            .get("admin_product_delete")
            .expect("delete")
            .path(),
        "/admin/products/{id}/delete"
    );

    let empty = AdminRegistry::new();
    register_admin_routes(&mut RouteCollection::new(), &empty).expect("empty ok");
}

#[test]
fn register_routes_propagates_name_collision() {
    let mut registry = AdminRegistry::new();
    registry
        .register(AdminResource::new("product", "/admin/products"))
        .expect("register");
    let mut collection = RouteCollection::new();
    collection
        .add(Route::with_method(
            "admin_product_list",
            "/collision",
            Method::Get,
        ))
        .expect("preexisting");
    let err = register_admin_routes(&mut collection, &registry).expect_err("collision");
    assert!(err.to_string().contains("admin_product_list"));
}

#[test]
fn register_routes_propagates_show_name_collision() {
    assert_route_name_collision("admin_product_show");
}

#[test]
fn register_routes_propagates_new_name_collision() {
    assert_route_name_collision("admin_product_new");
}

#[test]
fn register_routes_propagates_create_name_collision() {
    assert_route_name_collision("admin_product_create");
}

#[test]
fn register_routes_propagates_edit_name_collision() {
    assert_route_name_collision("admin_product_edit");
}

#[test]
fn register_routes_propagates_update_name_collision() {
    assert_route_name_collision("admin_product_update");
}

#[test]
fn register_routes_propagates_delete_name_collision() {
    assert_route_name_collision("admin_product_delete");
}

#[test]
fn render_list_and_show_escape_html() {
    let resource = AdminResource::new("product", "/admin/products")
        .list_fields([AdminField::named("name"), AdminField::new("price", "Price")]);
    let rows = [AdminRow::new("1")
        .with("name", "<script>")
        .with("price", "10")];
    let list = render_list_html(&resource, &rows);
    assert!(list.contains("&lt;script&gt;"));
    assert!(!list.contains("<script>"));
    assert!(list.contains("/admin/products/1"));
    let show = render_show_html(&resource, &rows[0]);
    assert!(show.contains("&lt;script&gt;"));
    assert!(show.contains("back to list"));
    assert_eq!(rows[0].id(), "1");
    assert_eq!(rows[0].get("missing"), "");
}

#[test]
fn forms_new_edit_delete_with_csrf_and_handler() {
    let mgr = HmacCsrfTokenManager::new(b"test-secret-key-32bytes-minimum!!");
    let resource = AdminResource::new("product", "/admin/products")
        .list_fields([AdminField::named("name")])
        .form_fields([AdminField::named("name")]);

    let new_html = render_new_form_html(&resource, &mgr).expect("new");
    assert!(new_html.contains("serenade-admin-new"));
    assert!(new_html.contains(CSRF_FIELD_NAME));
    assert!(new_html.contains("name=\"name\""));

    let row = AdminRow::new("1").with("name", "Mug");
    let edit_html = render_edit_form_html(&resource, &row, &mgr).expect("edit");
    assert!(edit_html.contains("value=\"Mug\""));
    assert!(edit_html.contains("/admin/products/1/edit"));

    let delete_html = render_delete_form_html(&resource, "1", &mgr).expect("delete");
    assert!(delete_html.contains("serenade-admin-delete"));
    assert!(delete_html.contains("/admin/products/1/delete"));

    let mut form = build_resource_form(&resource, resource.new_path());
    form.prepare_csrf(&mgr).expect("csrf");
    let token = extract_csrf(form.render().expect("render").as_html());
    let body = format!("{CSRF_FIELD_NAME}={token}&name=Cup");
    let request = Request::new(Method::Post, "/admin/products/new").with_body(body);
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert_eq!(form.get("name"), Some("Cup"));

    let handler = MemoryHandler::default();
    let created = handler.create(&form.data()).expect("create");
    assert_eq!(created.get("name"), "Cup");
    assert_eq!(handler.list().expect("list").len(), 1);
    let updated = handler
        .update(
            created.id(),
            &HashMap::from([("name".to_owned(), "Bowl".to_owned())]),
        )
        .expect("update");
    assert_eq!(updated.get("name"), "Bowl");
    assert!(handler.delete(created.id()).expect("delete"));
    assert!(!handler.delete(created.id()).expect("missing"));
    assert!(handler.get("nope").expect("get").is_none());
}

#[test]
fn admin_extension_registers_empty_registry() {
    let (_config, container) =
        build_container(None, "test", &[&FrameworkExtension, &AdminExtension]).expect("container");
    let registry = container
        .get_as::<AdminRegistry>(ADMIN_REGISTRY_SERVICE)
        .expect("registry");
    assert_eq!(registry.resources(), []);
    assert_eq!(AdminExtension.alias(), ADMIN_BUNDLE);
    assert_eq!(AdminBundle.name(), ADMIN_BUNDLE);
    assert_eq!(AdminBundle.dependencies(), [FRAMEWORK_BUNDLE]);
}

#[test]
fn admin_field_accessors() {
    let named = AdminField::named("id");
    assert_eq!(named.name(), "id");
    assert_eq!(named.label(), "id");
    let labeled = AdminField::new("price", "Price");
    assert_eq!(labeled.name(), "price");
    assert_eq!(labeled.label(), "Price");
}

#[derive(Default)]
struct MemoryHandler {
    rows: Mutex<HashMap<String, AdminRow>>,
    next_id: Mutex<u64>,
}

impl AdminResourceHandler for MemoryHandler {
    fn list(&self) -> Result<Vec<AdminRow>, AdminError> {
        Ok(self.rows.lock().expect("lock").values().cloned().collect())
    }

    fn get(&self, id: &str) -> Result<Option<AdminRow>, AdminError> {
        Ok(self.rows.lock().expect("lock").get(id).cloned())
    }

    fn create(&self, data: &HashMap<String, String>) -> Result<AdminRow, AdminError> {
        let id = {
            let mut next = self.next_id.lock().expect("lock");
            *next += 1;
            next.to_string()
        };
        let mut row = AdminRow::new(&id);
        for (key, value) in data {
            row = row.with(key.clone(), value.clone());
        }
        self.rows.lock().expect("lock").insert(id, row.clone());
        Ok(row)
    }

    fn update(&self, id: &str, data: &HashMap<String, String>) -> Result<AdminRow, AdminError> {
        if self.rows.lock().expect("lock").get(id).is_none() {
            return Err(AdminError::NotFound { id: id.to_owned() });
        }
        let mut row = AdminRow::new(id);
        for (key, value) in data {
            row = row.with(key.clone(), value.clone());
        }
        self.rows
            .lock()
            .expect("lock")
            .insert(id.to_owned(), row.clone());
        Ok(row)
    }

    fn delete(&self, id: &str) -> Result<bool, AdminError> {
        Ok(self.rows.lock().expect("lock").remove(id).is_some())
    }
}

fn assert_route_name_collision(route_name: &str) {
    let mut registry = AdminRegistry::new();
    registry
        .register(AdminResource::new("product", "/admin/products"))
        .expect("register");
    let mut collection = RouteCollection::new();
    collection
        .add(Route::with_method(route_name, "/collision", Method::Get))
        .expect("preexisting");
    let err = register_admin_routes(&mut collection, &registry).expect_err("collision");
    assert!(
        err.to_string().contains(route_name),
        "expected {route_name} in {err}"
    );
}

fn extract_csrf(html: &str) -> String {
    let marker = format!("name=\"{CSRF_FIELD_NAME}\" value=\"");
    let start = html.find(&marker).expect("csrf field") + marker.len();
    let end = html[start..].find('"').expect("end quote") + start;
    html[start..end].to_owned()
}
