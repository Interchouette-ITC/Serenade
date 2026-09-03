//! Symfony-style `.env` file loading (via `dotenvy`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::ConfigError;

/// Loads `.env` files from `project_dir`.
///
/// Order (missing files are skipped):
/// 1. `.env`
/// 2. `.env.local` (skipped when `environment` is `prod`)
/// 3. `.env.{environment}`
/// 4. `.env.{environment}.local`
///
/// Later files override earlier ones. Variables already present in the process
/// environment before this call are never changed.
///
/// # Errors
///
/// Returns [`ConfigError::Dotenv`] when a present file fails to parse.
pub fn load_dotenv(project_dir: impl AsRef<Path>, environment: &str) -> Result<(), ConfigError> {
    let project_dir = project_dir.as_ref();
    let env = environment.trim();
    let preexisting: HashSet<String> = std::env::vars().map(|(key, _)| key).collect();

    let mut files = vec![PathBuf::from(".env")];
    if env != "prod" {
        files.push(PathBuf::from(".env.local"));
    }
    if !env.is_empty() {
        files.push(PathBuf::from(format!(".env.{env}")));
        files.push(PathBuf::from(format!(".env.{env}.local")));
    }

    for relative in files {
        let path = project_dir.join(&relative);
        if !path.is_file() {
            continue;
        }
        let iter = dotenvy::from_path_iter(&path).map_err(|source| ConfigError::Dotenv {
            path: path.clone(),
            message: source.to_string(),
        })?;
        for item in iter {
            let (key, value) = item.map_err(|source| ConfigError::Dotenv {
                path: path.clone(),
                message: source.to_string(),
            })?;
            if preexisting.contains(&key) {
                continue;
            }
            std::env::set_var(key, value);
        }
    }
    Ok(())
}
