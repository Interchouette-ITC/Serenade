//! App-owned persistence hooks for one admin resource.

use std::collections::HashMap;

use crate::{AdminError, AdminRow};

/// Load / save / delete for one [`crate::AdminResource`] (app implements).
pub trait AdminResourceHandler: Send + Sync {
    /// Lists rows for the index page.
    ///
    /// # Errors
    ///
    /// Returns [`AdminError`] when the app repository fails.
    fn list(&self) -> Result<Vec<AdminRow>, AdminError>;

    /// Loads one row by id.
    ///
    /// # Errors
    ///
    /// Returns [`AdminError`] when the app repository fails.
    fn get(&self, id: &str) -> Result<Option<AdminRow>, AdminError>;

    /// Creates a row from bound form data.
    ///
    /// # Errors
    ///
    /// Returns [`AdminError`] when create fails.
    fn create(&self, data: &HashMap<String, String>) -> Result<AdminRow, AdminError>;

    /// Updates a row from bound form data.
    ///
    /// # Errors
    ///
    /// Returns [`AdminError`] when update fails or the id is missing.
    fn update(&self, id: &str, data: &HashMap<String, String>) -> Result<AdminRow, AdminError>;

    /// Deletes a row by id. Returns whether a row was removed.
    ///
    /// # Errors
    ///
    /// Returns [`AdminError`] when delete fails.
    fn delete(&self, id: &str) -> Result<bool, AdminError>;
}
