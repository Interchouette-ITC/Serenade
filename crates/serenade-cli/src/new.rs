//! `serenade new` app scaffolding.

use std::path::{Path, PathBuf};

use include_dir::{include_dir, Dir};

use crate::error::CliError;

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// Options for [`scaffold_app`].
#[derive(Debug, Clone)]
pub struct NewOptions {
    /// Parent directory for the new app (default `.`).
    pub path: PathBuf,
    /// Overwrite an existing destination tree.
    pub force: bool,
}

impl Default for NewOptions {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            force: false,
        }
    }
}

/// Creates a new Serenade app skeleton under `options.path / name`.
///
/// # Errors
///
/// Invalid name, existing destination without `--force`, missing template, or IO.
pub fn scaffold_app(name: &str, options: &NewOptions) -> Result<PathBuf, CliError> {
    validate_package_name(name)?;
    let dest_root = options.path.join(name);
    prepare_destination(&dest_root, options.force)?;

    let template = TEMPLATES
        .get_dir("app")
        .ok_or_else(|| CliError::MissingAsset("templates/app".to_owned()))?;
    write_dir_recursive(template, &dest_root, name)?;
    Ok(dest_root)
}

fn validate_package_name(name: &str) -> Result<(), CliError> {
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        || name.starts_with('-')
        || name.ends_with('-')
    {
        return Err(CliError::InvalidName(name.to_owned()));
    }
    Ok(())
}

fn prepare_destination(dest_root: &Path, force: bool) -> Result<(), CliError> {
    if !dest_root.exists() {
        std::fs::create_dir_all(dest_root)?;
        return Ok(());
    }
    let is_empty = std::fs::read_dir(dest_root)?.next().is_none();
    if is_empty {
        return Ok(());
    }
    if force {
        std::fs::remove_dir_all(dest_root)?;
        std::fs::create_dir_all(dest_root)?;
        return Ok(());
    }
    Err(CliError::DestinationExists(dest_root.to_path_buf()))
}

fn write_dir_recursive(
    dir: &Dir<'_>,
    dest_root: &Path,
    package_name: &str,
) -> Result<(), CliError> {
    for file in dir.files() {
        let relative = file
            .path()
            .strip_prefix("app")
            .unwrap_or_else(|_| file.path());
        let dest = dest_root.join(relative);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = substitute(file.contents(), package_name)?;
        std::fs::write(&dest, contents)?;
    }
    for child in dir.dirs() {
        write_dir_recursive(child, dest_root, package_name)?;
    }
    Ok(())
}

fn substitute(contents: &[u8], package_name: &str) -> Result<Vec<u8>, CliError> {
    let text = std::str::from_utf8(contents)
        .map_err(|_| CliError::InvalidRecipe("template file is not utf-8".to_owned()))?;
    Ok(text.replace("__PACKAGE_NAME__", package_name).into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn scaffolds_expected_files() {
        let dir = tempdir().expect("tempdir");
        let options = NewOptions {
            path: dir.path().to_path_buf(),
            force: false,
        };
        let root = scaffold_app("hello_app", &options).expect("scaffold");
        assert!(root.join("Cargo.toml").is_file());
        assert!(root.join("src/main.rs").is_file());
        assert!(root.join("src/bin/console.rs").is_file());
        assert!(root.join("config/packages/framework.toml").is_file());
        assert!(root.join(".env.example").is_file());
        let cargo = std::fs::read_to_string(root.join("Cargo.toml")).expect("read");
        assert!(cargo.contains("name = \"hello_app\""));
        assert!(!cargo.contains("__PACKAGE_NAME__"));
    }

    #[test]
    fn refuses_non_empty_without_force() {
        let dir = tempdir().expect("tempdir");
        let options = NewOptions {
            path: dir.path().to_path_buf(),
            force: false,
        };
        scaffold_app("once", &options).expect("first");
        let err = scaffold_app("once", &options).expect_err("second");
        assert!(matches!(err, CliError::DestinationExists(_)));
    }

    #[test]
    fn rejects_bad_name() {
        assert!(matches!(
            validate_package_name(""),
            Err(CliError::InvalidName(_))
        ));
        assert!(matches!(
            validate_package_name("bad name"),
            Err(CliError::InvalidName(_))
        ));
    }
}
