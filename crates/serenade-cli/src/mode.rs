//! Discovery and selection of example apps registered by Serenade mode.

use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;
use walkdir::WalkDir;

/// A Cargo package registered as an app for a Serenade mode.
#[derive(Debug)]
pub struct ModeApp {
    /// User-facing app name.
    pub name: String,
    /// Registered mode, such as `html` or `spa`.
    pub mode: String,
    /// Directory containing the app manifest.
    pub directory: PathBuf,
}

#[derive(Deserialize)]
struct CargoManifest {
    package: Option<Package>,
}

#[derive(Deserialize)]
struct Package {
    metadata: Option<PackageMetadata>,
}

#[derive(Deserialize)]
struct PackageMetadata {
    serenade: Option<SerenadeMetadata>,
}

#[derive(Deserialize)]
struct SerenadeMetadata {
    name: String,
    mode: String,
}

/// Finds the repository root by walking upward to a directory containing `examples/`.
///
/// # Errors
///
/// Returns an error when no ancestor of `start` contains an `examples/` directory.
pub fn find_examples_root(start: &Path) -> anyhow::Result<PathBuf> {
    start
        .ancestors()
        .find(|path| path.join("examples").is_dir())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            anyhow!(
                "could not find an examples/ directory from {}",
                start.display()
            )
        })
}

/// Reads registered apps from Cargo package metadata below `examples/`.
///
/// # Errors
///
/// Returns an error if a manifest cannot be scanned, read, or parsed.
pub fn discover_apps(examples: &Path) -> anyhow::Result<Vec<ModeApp>> {
    let mut apps = Vec::new();
    for entry in WalkDir::new(examples).into_iter().filter_entry(|entry| {
        !entry.file_type().is_dir()
            || !matches!(entry.file_name().to_str(), Some("target" | "dist"))
    }) {
        let entry = entry.with_context(|| format!("failed to scan {}", examples.display()))?;
        if !entry.file_type().is_file() || entry.file_name() != "Cargo.toml" {
            continue;
        }

        let contents = std::fs::read_to_string(entry.path())
            .with_context(|| format!("failed to read {}", entry.path().display()))?;
        let manifest: CargoManifest = toml::from_str(&contents)
            .with_context(|| format!("failed to parse {}", entry.path().display()))?;
        let Some(metadata) = manifest
            .package
            .and_then(|package| package.metadata)
            .and_then(|metadata| metadata.serenade)
        else {
            continue;
        };

        let directory = entry
            .path()
            .parent()
            .ok_or_else(|| {
                anyhow!(
                    "Cargo.toml has no parent directory: {}",
                    entry.path().display()
                )
            })?
            .to_path_buf();
        apps.push(ModeApp {
            name: metadata.name,
            mode: metadata.mode,
            directory,
        });
    }
    Ok(apps)
}

/// Selects a named/default app or every app registered for `mode`.
///
/// # Errors
///
/// Returns an error if no app matches the selection or a named app has another mode.
pub fn select_apps<'a>(
    apps: &'a [ModeApp],
    mode: &str,
    default_app: &str,
    all: bool,
    name: Option<&str>,
) -> anyhow::Result<Vec<&'a ModeApp>> {
    if all {
        let selected: Vec<_> = apps.iter().filter(|app| app.mode == mode).collect();
        if selected.is_empty() {
            bail!("no apps registered with mode = {mode:?}");
        }
        return Ok(selected);
    }

    let name = name.unwrap_or(default_app);
    let app = apps
        .iter()
        .find(|app| app.name == name)
        .ok_or_else(|| anyhow!("no app named `{name}` is registered"))?;
    if app.mode != mode {
        bail!("app `{name}` has mode = {:?}, expected {mode:?}", app.mode);
    }
    Ok(vec![app])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(name: &str, mode: &str) -> ModeApp {
        ModeApp {
            name: name.to_owned(),
            mode: mode.to_owned(),
            directory: PathBuf::from(name),
        }
    }

    #[test]
    fn defaults_to_named_app_and_rejects_wrong_mode() {
        let apps = [app("demo-spa", "spa"), app("demo-html", "html")];
        let selected = select_apps(&apps, "spa", "demo-spa", false, None)
            .expect("default SPA app should be selected");
        assert_eq!(selected[0].name, "demo-spa");

        let error = select_apps(&apps, "spa", "demo-spa", false, Some("demo-html"))
            .expect_err("wrong mode should fail");
        assert!(error.to_string().contains("expected \"spa\""));
    }

    #[test]
    fn all_selects_every_app_with_requested_mode() {
        let apps = [
            app("demo-html", "html"),
            app("demo-spa", "spa"),
            app("another-spa", "spa"),
        ];
        let selected = select_apps(&apps, "spa", "demo-spa", true, None)
            .expect("--all should find all apps in the mode");
        assert_eq!(
            selected
                .iter()
                .map(|app| app.name.as_str())
                .collect::<Vec<_>>(),
            ["demo-spa", "another-spa"]
        );
    }

    #[test]
    fn example_modes_are_registered_from_cargo_metadata() {
        let root = find_examples_root(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("Serenade repository root should be discoverable");
        let apps = discover_apps(&root.join("examples"))
            .expect("example app manifests should be readable");

        assert!(
            apps.iter()
                .any(|app| app.name == "demo-html" && app.mode == "html")
        );
        assert!(
            apps.iter()
                .any(|app| app.name == "demo-spa" && app.mode == "spa")
        );
    }
}
