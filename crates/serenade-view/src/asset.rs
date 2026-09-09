//! Asset URL helper.

use crate::ViewError;

/// Base path for application assets (default `/assets`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetConfig {
    base: String,
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            base: "/assets".to_owned(),
        }
    }
}

impl AssetConfig {
    /// Configures the asset URL prefix (for example `/static` or `/assets`).
    #[must_use]
    pub fn with_base(base: impl Into<String>) -> Self {
        Self { base: base.into() }
    }

    /// Joins `relative` under the configured base.
    ///
    /// Rejects `..`, scheme-like segments (`:`), and empty traversal abuse.
    /// An optional `?query` suffix is preserved for cache-busting.
    ///
    /// # Errors
    ///
    /// Returns [`ViewError`] when `relative` is unsafe.
    pub fn join(&self, relative: &str) -> Result<String, ViewError> {
        let trimmed = relative.trim().trim_start_matches('/');
        if trimmed.is_empty() {
            return Ok(normalize_base(&self.base));
        }
        let (file_part, query) = match trimmed.split_once('?') {
            Some((file, q)) => (file, Some(q)),
            None => (trimmed, None),
        };
        if file_part.is_empty() || file_part.contains("..") || file_part.contains(':') {
            return Err(ViewError::msg(format!("unsafe asset path `{relative}`")));
        }
        for part in file_part.split('/') {
            if part.is_empty() || part == "." {
                return Err(ViewError::msg(format!("unsafe asset path `{relative}`")));
            }
        }
        let base = normalize_base(&self.base);
        let joined = format!("{base}/{file_part}");
        Ok(match query {
            Some(q) => format!("{joined}?{q}"),
            None => joined,
        })
    }
}

/// Builds an asset URL under the default `/assets` base.
///
/// # Errors
///
/// Returns [`ViewError`] when `relative` is unsafe.
pub fn asset(relative: &str) -> Result<String, ViewError> {
    AssetConfig::default().join(relative)
}

fn normalize_base(base: &str) -> String {
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('/') {
        trimmed.to_owned()
    } else {
        format!("/{trimmed}")
    }
}
