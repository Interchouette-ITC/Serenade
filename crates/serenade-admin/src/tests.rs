use serenade_bundle::{
    BundleInterface, Extension, FRAMEWORK_BUNDLE, FrameworkExtension, build_container,
};
use serenade_http::{Method, Route, RouteCollection};

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
fn resource_paths_and_show_fields() {
    let with_slash = AdminResource::new("product", "/admin/products/")
        .list_fields([AdminField::named("name")])
        .show_fields([AdminField::new("sku", "SKU")]);
    assert_eq!(with_slash.path_prefix(), "/admin/products/");
    assert_eq!(with_slash.list().len(), 1);
    assert_eq!(with_slash.show().len(), 1);
    assert_eq!(with_slash.show()[0].name(), "sku");
    assert_eq!(with_slash.list_path(), "/admin/products");
    assert_eq!(with_slash.show_path(), "/admin/products/{id}");
    assert_eq!(with_slash.list_route_name(), "admin_product_list");
    assert_eq!(with_slash.show_route_name(), "admin_product_show");

    let fallback =
        AdminResource::new("cat", "admin/categories").list_fields([AdminField::named("title")]);
    assert_eq!(fallback.list_path(), "/admin/categories");
    assert_eq!(fallback.show().len(), 1);
    assert_eq!(fallback.show()[0].name(), "title");

    let root = AdminResource::new("root", "/");
    assert_eq!(root.list_path(), "/");
    assert_eq!(root.show_path(), "/{id}");

    let empty_prefix = AdminResource::new("empty", "");
    assert_eq!(empty_prefix.list_path(), "/");
}

#[test]
fn register_routes_list_and_show() {
    let mut registry = AdminRegistry::new();
    registry
        .register(
            AdminResource::new("product", "/admin/products")
                .list_fields([AdminField::named("name"), AdminField::new("price", "Price")]),
        )
        .expect("register");
    let mut collection = RouteCollection::new();
    register_admin_routes(&mut collection, &registry).expect("routes");
    let list = collection.get("admin_product_list").expect("list");
    assert_eq!(list.path(), "/admin/products");
    let show = collection.get("admin_product_show").expect("show");
    assert_eq!(show.path(), "/admin/products/{id}");

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
    let mut registry = AdminRegistry::new();
    registry
        .register(AdminResource::new("product", "/admin/products"))
        .expect("register");
    let mut collection = RouteCollection::new();
    collection
        .add(Route::with_method(
            "admin_product_show",
            "/collision-show",
            Method::Get,
        ))
        .expect("preexisting show");
    let err = register_admin_routes(&mut collection, &registry).expect_err("show collision");
    assert!(err.to_string().contains("admin_product_show"));
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
