//! Register list/show routes for a registry.

use serenade_http::{HttpError, Method, Route, RouteCollection};

use crate::AdminRegistry;

/// Adds GET list + show routes for every resource in `registry`.
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
            resource.show_route_name(),
            resource.show_path(),
            Method::Get,
        ))?;
    }
    Ok(())
}
