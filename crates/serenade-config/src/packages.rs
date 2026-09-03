//! Bundle package loader for `config/packages/*`.

use std::path::Path;

use crate::{Config, ConfigError};

/// Loads and merges YAML/TOML files from a packages directory (sorted by file name).
///
/// # Errors
///
/// Returns [`ConfigError`] when the directory cannot be read or a file fails to parse.
pub fn load_packages(dir: impl AsRef<Path>) -> Result<Config, ConfigError> {
    let dir = dir.as_ref();
    let mut entries = Vec::new();
    let read = std::fs::read_dir(dir).map_err(|source| ConfigError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in read {
        let entry = entry.map_err(|source| ConfigError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
            continue;
        };
        if !matches!(ext.to_ascii_lowercase().as_str(), "yaml" | "yml" | "toml") {
            continue;
        }
        entries.push(path);
    }
    entries.sort();
    let mut merged = Config::empty();
    for path in entries {
        merged = merged.merged(&Config::from_path(&path)?);
    }
    Ok(merged)
}
