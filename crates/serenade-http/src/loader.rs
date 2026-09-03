//! Bundle (and other) route loaders.

use crate::{HttpError, RouteCollection};

/// Loads routes into a [`RouteCollection`].
///
/// Bundles implement this to contribute routes during kernel compile/boot.
pub trait RouteLoader {
    /// Appends routes to `collection`.
    ///
    /// # Errors
    ///
    /// Returns [`HttpError`] when registration fails (duplicate name, invalid path, …).
    fn load(&self, collection: &mut RouteCollection) -> Result<(), HttpError>;
}

impl<F> RouteLoader for F
where
    F: Fn(&mut RouteCollection) -> Result<(), HttpError>,
{
    fn load(&self, collection: &mut RouteCollection) -> Result<(), HttpError> {
        self(collection)
    }
}

/// Runs every loader in order into a fresh collection.
///
/// # Errors
///
/// Returns the first loader error.
pub fn load_routes(loaders: &[&dyn RouteLoader]) -> Result<RouteCollection, HttpError> {
    let mut collection = RouteCollection::new();
    for loader in loaders {
        loader.load(&mut collection)?;
    }
    Ok(collection)
}
