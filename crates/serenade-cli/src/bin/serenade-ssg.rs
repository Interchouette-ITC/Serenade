//! Build registered Serenade SSG apps with Trunk and prerender their routes.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, bail};
use clap::Parser;
use serde::Deserialize;
use serenade_cli::mode::{ModeApp, discover_apps, find_examples_root, select_apps};

#[derive(Debug, Parser)]
#[command(name = "serenade-ssg", about = "Build a Serenade SSG app with Trunk")]
struct Cli {
    /// Build every app registered with mode = "ssg".
    #[arg(long, conflicts_with_all = ["app", "app_dir", "manifest_path"])]
    all: bool,
    /// Build the app named APP (defaults to demo-ssg).
    #[arg(conflicts_with_all = ["app_dir", "manifest_path"])]
    app: Option<String>,
    /// Build the Cargo package in this application directory.
    #[arg(long, conflicts_with = "manifest_path")]
    app_dir: Option<PathBuf>,
    /// Build the Cargo package described by this manifest.
    #[arg(long)]
    manifest_path: Option<PathBuf>,
    /// Build the hydration bundle and prerenderer in release mode.
    #[arg(long)]
    release: bool,
}

#[derive(Deserialize)]
struct CargoManifest {
    package: Option<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
}

fn external_app(path: &Path) -> anyhow::Result<ModeApp> {
    let manifest = if path.is_dir() {
        path.join("Cargo.toml")
    } else {
        path.to_path_buf()
    };
    let manifest = manifest
        .canonicalize()
        .with_context(|| format!("failed to find application manifest {}", manifest.display()))?;
    if manifest.file_name().and_then(|name| name.to_str()) != Some("Cargo.toml")
        || !manifest.is_file()
    {
        bail!(
            "application manifest must be a Cargo.toml file: {}",
            manifest.display()
        );
    }
    let contents = std::fs::read_to_string(&manifest)
        .with_context(|| format!("failed to read {}", manifest.display()))?;
    let parsed: CargoManifest = toml::from_str(&contents)
        .with_context(|| format!("failed to parse {}", manifest.display()))?;
    let package = parsed.package.ok_or_else(|| {
        anyhow::anyhow!(
            "{} is a workspace manifest, not an application package; specify the application's Cargo.toml",
            manifest.display()
        )
    })?;
    let directory = manifest
        .parent()
        .context("application manifest has no parent directory")?
        .to_path_buf();
    Ok(ModeApp {
        name: package.name,
        mode: "ssg".to_owned(),
        directory,
    })
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    if let Some(directory) = &cli.app_dir
        && !directory.is_dir()
    {
        bail!(
            "application directory does not exist: {}",
            directory.display()
        );
    }
    if let Some(manifest) = &cli.manifest_path
        && !manifest.is_file()
    {
        bail!(
            "application manifest does not exist: {}",
            manifest.display()
        );
    }
    let external = cli.app_dir.as_deref().or(cli.manifest_path.as_deref());
    let apps = if let Some(path) = external {
        vec![external_app(path)?]
    } else {
        let root = find_examples_root(&std::env::current_dir()?)?;
        discover_apps(&root.join("examples"))?
    };
    let selected = if external.is_some() {
        apps.iter().collect()
    } else {
        select_apps(&apps, "ssg", "demo-ssg", cli.all, cli.app.as_deref())?
    };

    let profile: &[&str] = if cli.release { &["--release"] } else { &[] };
    for app in selected {
        println!("Building {} hydration bundle with Trunk", app.name);
        let trunk_args = [profile, &["--features", "hydrate"]].concat();
        serenade_cli::trunk::build(&app.directory, &trunk_args).map_err(|error| {
            anyhow::anyhow!("failed to build SSG app `{}`: {error:#}", app.name)
        })?;
        let dist = serenade_cli::html_output::dist_directory_for_build(&app.directory, &[])?;
        let manifest = dist.join(serenade_view::TRUNK_ASSET_MANIFEST_FILE);

        println!("Prerendering {} routes", app.name);
        let status = Command::new("cargo")
            .args(["run", "--locked", "--manifest-path"])
            .arg(app.directory.join("Cargo.toml"))
            .args(profile)
            .args([
                "--bin",
                "prerender",
                "--no-default-features",
                "--features",
                "ssr",
            ])
            .current_dir(&app.directory)
            .env(serenade_view::STATIC_OUTPUT_DIR_ENV, &dist)
            .env(serenade_view::TRUNK_ASSET_MANIFEST_ENV, &manifest)
            .status()
            .with_context(|| format!("failed to start prerenderer for `{}`", app.name))?;
        if !status.success() {
            bail!(
                "SSG prerenderer for app `{}` exited with status {status}",
                app.name
            );
        }
        let minified = serenade_cli::html_output::minify_html_files(&dist)?;
        println!("Minified {minified} HTML document(s) for {}", app.name);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_package_can_be_selected_by_directory_or_manifest() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let manifest = temp.path().join("Cargo.toml");
        std::fs::write(&manifest, "[package]\nname = \"external-site\"\n")
            .expect("write package manifest");

        for path in [temp.path(), manifest.as_path()] {
            let app = external_app(path).expect("external app");
            assert_eq!(app.name, "external-site");
            assert_eq!(app.directory, temp.path().canonicalize().unwrap());
        }
    }

    #[test]
    fn workspace_and_missing_manifests_are_reported() {
        let temp = tempfile::tempdir().expect("temporary directory");
        std::fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .expect("write workspace manifest");
        assert!(
            external_app(temp.path())
                .unwrap_err()
                .to_string()
                .contains("workspace manifest")
        );
        assert!(
            external_app(&temp.path().join("missing"))
                .unwrap_err()
                .to_string()
                .contains("failed to find application manifest")
        );
    }

    #[test]
    fn external_path_conflicts_with_registered_app_selection() {
        assert!(Cli::try_parse_from(["serenade-ssg", "--app-dir", ".", "--all"]).is_err());
        assert!(
            Cli::try_parse_from(["serenade-ssg", "--manifest-path", "Cargo.toml", "app"]).is_err()
        );
        assert!(
            Cli::try_parse_from([
                "serenade-ssg",
                "--app-dir",
                ".",
                "--manifest-path",
                "Cargo.toml"
            ])
            .is_err()
        );
    }
}
