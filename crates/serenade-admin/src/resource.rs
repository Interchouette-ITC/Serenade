//! Admin resource descriptor (one CRUD entity surface).

use crate::AdminField;

/// Config for one admin resource (list / show in this slice).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminResource {
    name: String,
    path_prefix: String,
    list_fields: Vec<AdminField>,
    show_fields: Vec<AdminField>,
}

impl AdminResource {
    /// Builds a resource. `path_prefix` should start with `/` (for example `/admin/products`).
    #[must_use]
    pub fn new(name: impl Into<String>, path_prefix: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path_prefix: path_prefix.into(),
            list_fields: Vec::new(),
            show_fields: Vec::new(),
        }
    }

    /// Sets list columns.
    #[must_use]
    pub fn list_fields(mut self, fields: impl IntoIterator<Item = AdminField>) -> Self {
        self.list_fields = fields.into_iter().collect();
        self
    }

    /// Sets show fields (defaults to list fields when empty at render time).
    #[must_use]
    pub fn show_fields(mut self, fields: impl IntoIterator<Item = AdminField>) -> Self {
        self.show_fields = fields.into_iter().collect();
        self
    }

    /// Stable resource id (used in route names).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// URL prefix without trailing slash preference: stored as given.
    #[must_use]
    pub fn path_prefix(&self) -> &str {
        &self.path_prefix
    }

    /// List columns.
    #[must_use]
    pub fn list(&self) -> &[AdminField] {
        &self.list_fields
    }

    /// Show fields; falls back to list fields when unset.
    #[must_use]
    pub fn show(&self) -> &[AdminField] {
        if self.show_fields.is_empty() {
            &self.list_fields
        } else {
            &self.show_fields
        }
    }

    /// Route name for list GET.
    #[must_use]
    pub fn list_route_name(&self) -> String {
        format!("admin_{}_list", self.name)
    }

    /// Route name for show GET.
    #[must_use]
    pub fn show_route_name(&self) -> String {
        format!("admin_{}_show", self.name)
    }

    /// List path (`path_prefix`).
    #[must_use]
    pub fn list_path(&self) -> String {
        normalize_prefix(&self.path_prefix)
    }

    /// Show path (`path_prefix/{id}`).
    #[must_use]
    pub fn show_path(&self) -> String {
        let prefix = normalize_prefix(&self.path_prefix);
        if prefix == "/" {
            "/{id}".to_owned()
        } else {
            format!("{prefix}/{{id}}")
        }
    }
}

fn normalize_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_owned()
    } else if trimmed.starts_with('/') {
        trimmed.to_owned()
    } else {
        format!("/{trimmed}")
    }
}
