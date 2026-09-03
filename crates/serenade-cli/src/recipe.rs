//! Recipe load and apply.

use std::path::{Path, PathBuf};
use std::process::Command;

use include_dir::{include_dir, Dir, File};
use serde::Deserialize;

use crate::error::CliError;

static RECIPES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/recipes");

/// Options for [`apply_recipe`].
#[derive(Debug, Clone)]
pub struct ApplyOptions {
    /// Application root (default `.`).
    pub root: PathBuf,
    /// Overwrite existing destination files.
    pub force: bool,
    /// Skip running `cargo add` for declared dependencies.
    pub no_cargo: bool,
}

impl Default for ApplyOptions {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            force: false,
            no_cargo: false,
        }
    }
}

/// Declared Cargo dependency hint (resolved by Cargo).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CargoDependency {
    /// Crate name for `cargo add`.
    #[serde(rename = "crate")]
    pub crate_name: String,
    /// Git repository URL.
    pub git: Option<String>,
    /// Git branch.
    pub branch: Option<String>,
}

/// Cargo section of a recipe.
#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct CargoSection {
    /// Dependencies to add via `cargo add`.
    #[serde(default)]
    pub dependencies: Vec<CargoDependency>,
}

/// File copy entry.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RecipeFile {
    /// Path relative to the recipe directory.
    pub src: String,
    /// Path relative to the application root.
    pub dest: String,
}

/// Post-apply hints printed to the user.
#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
pub struct RecipeHints {
    /// Bundle names to register.
    #[serde(default)]
    pub bundles: Vec<String>,
    /// Free-form note.
    #[serde(default)]
    pub note: Option<String>,
}

/// Parsed Flex-like recipe.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Recipe {
    /// Stable recipe id (`framework`, `security`, …).
    pub id: String,
    /// Short description.
    pub description: String,
    /// Optional Cargo dependency declarations.
    #[serde(default)]
    pub cargo: CargoSection,
    /// Files to copy into the app.
    #[serde(default)]
    pub files: Vec<RecipeFile>,
    /// Human hints after apply.
    #[serde(default)]
    pub hints: RecipeHints,
}

/// Lists embedded recipe ids with descriptions.
#[must_use]
pub fn list_recipes() -> Vec<(String, String)> {
    let mut items = Vec::new();
    for entry in RECIPES.dirs() {
        let Some(file) = recipe_meta_file(entry) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(file.contents()) else {
            continue;
        };
        let Ok(recipe) = toml::from_str::<Recipe>(text) else {
            continue;
        };
        items.push((recipe.id, recipe.description));
    }
    items.sort_by(|left, right| left.0.cmp(&right.0));
    items
}

/// Loads an embedded recipe by id.
///
/// # Errors
///
/// Unknown id or invalid metadata.
pub fn load_recipe(id: &str) -> Result<(Recipe, &'static Dir<'static>), CliError> {
    for entry in RECIPES.dirs() {
        let Some(file) = recipe_meta_file(entry) else {
            continue;
        };
        let text = std::str::from_utf8(file.contents()).map_err(|_| {
            CliError::InvalidRecipe(format!(
                "non-utf8 recipe.toml in {}",
                entry.path().display()
            ))
        })?;
        let recipe: Recipe =
            toml::from_str(text).map_err(|error| CliError::InvalidRecipe(error.to_string()))?;
        if recipe.id == id {
            return Ok((recipe, entry));
        }
    }
    Err(CliError::UnknownRecipe(id.to_owned()))
}

/// Applies a recipe into `options.root`.
///
/// # Errors
///
/// Missing assets, refused overwrite, IO, or `cargo add` failure.
pub fn apply_recipe(id: &str, options: &ApplyOptions) -> Result<Recipe, CliError> {
    let (recipe, dir) = load_recipe(id)?;
    for file in &recipe.files {
        let asset = recipe_asset(dir, &file.src).ok_or_else(|| {
            CliError::MissingAsset(format!("{}/{}", dir.path().display(), file.src))
        })?;
        let dest = options.root.join(&file.dest);
        write_file(&dest, asset.contents(), options.force)?;
    }

    if !options.no_cargo && !recipe.cargo.dependencies.is_empty() {
        let manifest = options.root.join("Cargo.toml");
        if manifest.is_file() {
            run_cargo_add(&options.root, &recipe.cargo.dependencies)?;
        }
    }

    Ok(recipe)
}

fn recipe_meta_file(dir: &'static Dir<'static>) -> Option<&'static File<'static>> {
    find_file(dir, &|path| {
        path.file_name().is_some_and(|name| name == "recipe.toml")
    })
}

fn recipe_asset(dir: &'static Dir<'static>, src: &str) -> Option<&'static File<'static>> {
    let prefixed = dir.path().join(src);
    find_file(dir, &|path| {
        path == prefixed.as_path() || path.ends_with(src)
    })
}

fn find_file(
    dir: &'static Dir<'static>,
    predicate: &dyn Fn(&Path) -> bool,
) -> Option<&'static File<'static>> {
    for file in dir.files() {
        if predicate(file.path()) {
            return Some(file);
        }
    }
    for child in dir.dirs() {
        if let Some(file) = find_file(child, predicate) {
            return Some(file);
        }
    }
    None
}

fn write_file(dest: &Path, contents: &[u8], force: bool) -> Result<(), CliError> {
    if dest.exists() && !force {
        return Err(CliError::FileExists(dest.to_path_buf()));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(dest, contents)?;
    Ok(())
}

fn run_cargo_add(root: &Path, deps: &[CargoDependency]) -> Result<(), CliError> {
    for dep in deps {
        let mut command = Command::new("cargo");
        command.arg("add").arg(&dep.crate_name).current_dir(root);
        if let Some(git) = &dep.git {
            command.arg("--git").arg(git);
        }
        if let Some(branch) = &dep.branch {
            command.arg("--branch").arg(branch);
        }
        let status = command
            .status()
            .map_err(|error| CliError::Cargo(format!("failed to spawn cargo: {error}")))?;
        if !status.success() {
            return Err(CliError::Cargo(format!(
                "`cargo add {}` exited with {status}",
                dep.crate_name
            )));
        }
    }
    Ok(())
}

/// Prints post-apply hints to stdout.
pub fn print_hints(recipe: &Recipe) {
    if !recipe.hints.bundles.is_empty() {
        println!("bundles: {}", recipe.hints.bundles.join(", "));
    }
    if let Some(note) = &recipe.hints.note {
        println!("{note}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn lists_framework_and_security() {
        let ids: Vec<_> = list_recipes().into_iter().map(|(id, _)| id).collect();
        assert!(ids.contains(&"framework".to_owned()));
        assert!(ids.contains(&"security".to_owned()));
    }

    #[test]
    fn loads_framework_recipe() {
        let (recipe, _) = load_recipe("framework").expect("framework");
        assert_eq!(recipe.id, "framework");
        assert!(
            !recipe.files.is_empty(),
            "framework recipe should list files"
        );
    }

    #[test]
    fn apply_writes_and_refuses_overwrite() {
        let dir = tempdir().expect("tempdir");
        let options = ApplyOptions {
            root: dir.path().to_path_buf(),
            force: false,
            no_cargo: true,
        };
        apply_recipe("security", &options).expect("first apply");
        let dest = dir.path().join("config/packages/security.toml");
        assert!(dest.is_file());
        let err = apply_recipe("security", &options).expect_err("overwrite");
        assert!(matches!(err, CliError::FileExists(_)));
        let forced = ApplyOptions {
            force: true,
            ..options
        };
        apply_recipe("security", &forced).expect("force");
    }

    #[test]
    fn unknown_recipe_errors() {
        assert!(matches!(
            load_recipe("nope"),
            Err(CliError::UnknownRecipe(_))
        ));
    }
}
