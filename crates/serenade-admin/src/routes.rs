//! Register CRUD routes for a registry.

use serenade_http::{HttpError, Method, Route, RouteCollection};

use crate::AdminRegistry;

/// Adds list / show / new / create / edit / update / delete routes for every resource.
///
/// Static `…/new` routes are registered before `{id}` so path matching stays correct.
///
/// # Errors
///
/// Propagates [`HttpError`] when a route name collides.
pub fn register_admin_routes(
    collection: &mut RouteCollection,
    registry: &AdminRegistry,
) -> Result<(), HttpError> {
    for resource in registry.resources() {
        collection.add(Route::with_method(
            resource.list_route_name(),
            resource.list_path(),
            Method::Get,
        ))?;
        collection.add(Route::with_method(
            resource.new_route_name(),
            resource.new_path(),
            Method::Get,
        ))?;
        collection.add(Route::with_method(
            resource.create_route_name(),
            resource.new_path(),
            Method::Post,
        ))?;
        collection.add(Route::with_method(
            resource.show_route_name(),
            resource.show_path(),
            Method::Get,
        ))?;
        collection.add(Route::with_method(
            resource.edit_route_name(),
            resource.edit_path(),
            Method::Get,
        ))?;
        collection.add(Route::with_method(
            resource.update_route_name(),
            resource.edit_path(),
            Method::Post,
        ))?;
        collection.add(Route::with_method(
            resource.delete_route_name(),
            resource.delete_path(),
            Method::Post,
        ))?;
    }
    Ok(())
}
