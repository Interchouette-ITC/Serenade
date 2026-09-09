//! Reverse URL generation.

use serenade_http::{HttpError, RouteCollection};

/// Builds a path for the named route (Twig `path()` analogue).
///
/// # Errors
///
/// Propagates [`RouteCollection::generate`] failures.
pub fn path(
    routes: &RouteCollection,
    name: &str,
    params: &[(&str, &str)],
) -> Result<String, HttpError> {
    routes.generate(name, params)
}
