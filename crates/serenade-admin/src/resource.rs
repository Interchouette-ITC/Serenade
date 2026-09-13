//! Admin resource descriptor (one CRUD entity surface).

use crate::AdminField;

/// Config for one admin resource (list / show / form).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminResource {
    name: String,
    path_prefix: String,
    list_fields: Vec<AdminField>,
    show_fields: Vec<AdminField>,
    form_fields: Vec<AdminField>,
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
            form_fields: Vec::new(),
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

    /// Sets create/edit form fields (defaults to list fields when empty).
    #[must_use]
    pub fn form_fields(mut self, fields: impl IntoIterator<Item = AdminField>) -> Self {
        self.form_fields = fields.into_iter().collect();
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

    /// Form fields; falls back to list fields when unset.
    #[must_use]
    pub fn form(&self) -> &[AdminField] {
        if self.form_fields.is_empty() {
            &self.list_fields
        } else {
            &self.form_fields
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

    /// Route name for new GET.
    #[must_use]
    pub fn new_route_name(&self) -> String {
        format!("admin_{}_new", self.name)
    }

    /// Route name for create POST.
    #[must_use]
    pub fn create_route_name(&self) -> String {
        format!("admin_{}_create", self.name)
    }

    /// Route name for edit GET.
    #[must_use]
    pub fn edit_route_name(&self) -> String {
        format!("admin_{}_edit", self.name)
    }

    /// Route name for update POST.
    #[must_use]
    pub fn update_route_name(&self) -> String {
        format!("admin_{}_update", self.name)
    }

    /// Route name for delete POST.
    #[must_use]
    pub fn delete_route_name(&self) -> String {
        format!("admin_{}_delete", self.name)
    }

    /// List path (`path_prefix`).
    #[must_use]
    pub fn list_path(&self) -> String {
        normalize_prefix(&self.path_prefix)
    }

    /// New / create path (`path_prefix/new`).
    #[must_use]
    pub fn new_path(&self) -> String {
        join_prefix(&self.list_path(), "new")
    }

    /// Show path (`path_prefix/{id}`).
    #[must_use]
    pub fn show_path(&self) -> String {
        join_id_pattern(&self.list_path())
    }

    /// Edit / update path pattern (`path_prefix/{id}/edit`).
    #[must_use]
    pub fn edit_path(&self) -> String {
        join_prefix(&join_id_pattern(&self.list_path()), "edit")
    }

    /// Delete path pattern (`path_prefix/{id}/delete`).
    #[must_use]
    pub fn delete_path(&self) -> String {
        join_prefix(&join_id_pattern(&self.list_path()), "delete")
    }

    /// Concrete edit URL for `id`.
    #[must_use]
    pub fn edit_path_for(&self, id: &str) -> String {
        join_prefix(&join_id(&self.list_path(), id), "edit")
    }

    /// Concrete delete URL for `id`.
    #[must_use]
    pub fn delete_path_for(&self, id: &str) -> String {
        join_prefix(&join_id(&self.list_path(), id), "delete")
    }

    /// Concrete show URL for `id`.
    #[must_use]
    pub fn show_path_for(&self, id: &str) -> String {
        join_id(&self.list_path(), id)
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

fn join_prefix(prefix: &str, segment: &str) -> String {
    if prefix == "/" {
        format!("/{segment}")
    } else {
        format!("{prefix}/{segment}")
    }
}

fn join_id_pattern(prefix: &str) -> String {
    join_prefix(prefix, "{id}")
}

fn join_id(prefix: &str, id: &str) -> String {
    join_prefix(prefix, id)
}
