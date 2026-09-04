//! Bundle package loader for `config/packages/*`.

use std::path::{Path, PathBuf};

use crate::{Config, ConfigError};

/// Loads and merges YAML/TOML files from a packages directory (sorted by file name).
///
/// Only files directly in `dir` are loaded; subdirectories are ignored.
/// Prefer [`load_packages_for_env`] when an environment overlay is needed.
///
/// # Errors
///
/// Returns [`ConfigError`] when the directory cannot be read or a file fails to parse.
pub fn load_packages(dir: impl AsRef<Path>) -> Result<Config, ConfigError> {
    load_package_files(dir.as_ref())
}

/// Loads base packages then merges `packages/{environment}/` when that directory exists.
///
/// # Errors
///
/// Returns [`ConfigError`] when a packages directory cannot be read or a file fails to parse.
pub fn load_packages_for_env(
    dir: impl AsRef<Path>,
    environment: &str,
) -> Result<Config, ConfigError> {
    let dir = dir.as_ref();
    let mut merged = load_package_files(dir)?;
    let env = environment.trim();
    if env.is_empty() {
        return Ok(merged);
    }
    let overlay = dir.join(env);
    if overlay.is_dir() {
        merged = merged.merged(&load_package_files(&overlay)?);
    }
    Ok(merged)
}

fn load_package_files(dir: &Path) -> Result<Config, ConfigError> {
    let mut entries = list_package_files(dir)?;
    entries.sort();
    let mut merged = Config::empty();
    for path in entries {
        merged = merged.merged(&Config::from_path(&path)?);
    }
    Ok(merged)
}

fn list_package_files(dir: &Path) -> Result<Vec<PathBuf>, ConfigError> {
    let mut entries = Vec::new();
    let read = std::fs::read_dir(dir).map_err(|source| ConfigError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in read.flatten() {
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
    Ok(entries)
}
