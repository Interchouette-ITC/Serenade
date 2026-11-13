//! Post-processes static HTML output produced by frontend and SSG builds.

use std::path::Path;

use anyhow::{Context, bail};
use serde::Deserialize;
use serenade_view::TrunkAssetManifest;
use walkdir::WalkDir;

#[derive(Deserialize)]
struct TrunkConfig {
    build: Option<TrunkBuild>,
}

#[derive(Deserialize)]
struct TrunkBuild {
    dist: Option<std::path::PathBuf>,
}

/// Resolves Trunk's configured output directory, defaulting to `dist/`.
///
/// # Errors
///
/// Returns an error when an existing `Trunk.toml` cannot be read or parsed.
pub fn dist_directory(app: &Path) -> anyhow::Result<std::path::PathBuf> {
    dist_directory_with_override(app, None)
}

/// Resolves the build output directory, including a CLI or environment override.
///
/// Trunk's precedence is command-line `--dist`, then `TRUNK_BUILD_DIST`, then
/// `Trunk.toml`, then the default `dist/`.
///
/// # Errors
///
/// Returns an error when a CLI `--dist` option has no value or an existing
/// `Trunk.toml` cannot be read or parsed.
pub fn dist_directory_for_build(app: &Path, args: &[&str]) -> anyhow::Result<std::path::PathBuf> {
    let mut args_iter = args.iter();
    let mut cli_dist = None;
    while let Some(arg) = args_iter.next() {
        if *arg == "--dist" || *arg == "-d" {
            let value = args_iter
                .next()
                .ok_or_else(|| anyhow::anyhow!("{arg} requires an output directory"))?;
            cli_dist = Some(std::path::PathBuf::from(value));
        } else if let Some(value) = arg.strip_prefix("--dist=") {
            cli_dist = Some(std::path::PathBuf::from(value));
        }
    }

    let override_dist = cli_dist.or_else(|| std::env::var_os("TRUNK_BUILD_DIST").map(Into::into));
    dist_directory_with_override(app, override_dist)
}

fn dist_directory_with_override(
    app: &Path,
    override_dist: Option<std::path::PathBuf>,
) -> anyhow::Result<std::path::PathBuf> {
    if let Some(dist) = override_dist {
        return Ok(if dist.is_absolute() {
            dist
        } else {
            app.join(dist)
        });
    }

    let config_path = app.join("Trunk.toml");
    let config = match std::fs::read_to_string(&config_path) {
        Ok(contents) => Some(
            toml::from_str::<TrunkConfig>(&contents)
                .with_context(|| format!("failed to parse {}", config_path.display()))?,
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", config_path.display()));
        }
    };
    let dist = config
        .and_then(|config| config.build)
        .and_then(|build| build.dist)
        .unwrap_or_else(|| std::path::PathBuf::from("dist"));
    Ok(if dist.is_absolute() {
        dist
    } else {
        app.join(dist)
    })
}

/// Removes a previous Trunk manifest before starting a new build.
///
/// This prevents a failed or incompatible Trunk invocation from appearing to
/// have produced a fresh manifest by leaving a prior build's file in place.
///
/// # Errors
///
/// Returns an error when the old manifest cannot be inspected or removed.
pub fn clear_trunk_asset_manifest(dist: &Path) -> anyhow::Result<()> {
    let manifest = dist.join(serenade_view::TRUNK_ASSET_MANIFEST_FILE);
    match std::fs::symlink_metadata(&manifest) {
        Ok(metadata) if metadata.file_type().is_file() || metadata.file_type().is_symlink() => {
            std::fs::remove_file(&manifest).with_context(|| {
                format!(
                    "failed to remove stale asset manifest {}",
                    manifest.display()
                )
            })
        }
        Ok(_) => bail!(
            "Trunk asset manifest path is not a file: {}",
            manifest.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("failed to inspect asset manifest {}", manifest.display())),
    }
}

/// Loads the manifest from a completed Trunk build and validates all outputs.
///
/// # Errors
///
/// Returns an error if the manifest is missing, malformed, incompatible, or
/// references missing, stale, or unsafe artifacts.
pub fn validate_trunk_asset_manifest(dist: &Path) -> anyhow::Result<TrunkAssetManifest> {
    let path = dist.join(serenade_view::TRUNK_ASSET_MANIFEST_FILE);
    let manifest = TrunkAssetManifest::read(&path)
        .with_context(|| format!("failed to load Trunk asset manifest {}", path.display()))?;
    manifest
        .validate_output(dist)
        .with_context(|| format!("invalid Trunk build artifacts in {}", dist.display()))?;
    Ok(manifest)
}

/// Minifies every UTF-8 `.html` file below `dist`.
///
/// # Errors
///
/// Returns an error when the output directory is missing, scanning fails, a
/// document cannot be read, or writing its minified form fails.
pub fn minify_html_files(dist: &Path) -> anyhow::Result<usize> {
    if !dist.is_dir() {
        bail!("HTML output directory does not exist: {}", dist.display());
    }

    let mut count = 0;
    for entry in WalkDir::new(dist) {
        let entry = entry.with_context(|| format!("failed to scan {}", dist.display()))?;
        if !entry.file_type().is_file()
            || !entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("html"))
        {
            continue;
        }

        let path = entry.path();
        let html = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read HTML document {}", path.display()))?;
        let minified = serenade_minify::minify_html(&html)
            .with_context(|| format!("failed to minify HTML document {}", path.display()))?;
        std::fs::write(path, minified).with_context(|| {
            format!("failed to write minified HTML document {}", path.display())
        })?;
        count += 1;
    }

    if count == 0 {
        bail!("no HTML documents found in {}", dist.display());
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minifies_every_html_document_and_leaves_other_assets_untouched() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let nested = temp.path().join("about");
        std::fs::create_dir(&nested).expect("nested directory");
        let home = temp.path().join("index.html");
        let about = nested.join("index.html");
        let css = temp.path().join("site.css");
        std::fs::write(
            &home,
            "<!doctype html>\n<html><body><p> hello </p></body></html>",
        )
        .expect("home HTML");
        std::fs::write(&about, "<html>\n<body><p>world</p></body>\n</html>").expect("about HTML");
        std::fs::write(&css, " body { color: red; }").expect("CSS asset");

        assert_eq!(minify_html_files(temp.path()).expect("minify files"), 2);
        assert_eq!(
            std::fs::read_to_string(home).expect("read home"),
            "<!doctype html><html><body><p>hello</p></body></html>"
        );
        assert_eq!(
            std::fs::read_to_string(about).expect("read about"),
            "<html><body><p>world</p></body></html>"
        );
        assert_eq!(
            std::fs::read_to_string(css).expect("read CSS"),
            " body { color: red; }"
        );
    }

    #[test]
    fn rejects_missing_or_empty_output_directories() {
        let temp = tempfile::tempdir().expect("temporary directory");
        assert!(minify_html_files(&temp.path().join("missing")).is_err());
        assert!(minify_html_files(temp.path()).is_err());
    }

    #[test]
    fn resolves_trunk_dist_with_default_and_custom_paths() {
        let temp = tempfile::tempdir().expect("temporary directory");
        assert_eq!(
            dist_directory(temp.path()).expect("default dist"),
            temp.path().join("dist")
        );

        std::fs::write(
            temp.path().join("Trunk.toml"),
            "[build]\ndist = \"public/site\"\n",
        )
        .expect("Trunk config");
        assert_eq!(
            dist_directory(temp.path()).expect("configured dist"),
            temp.path().join("public/site")
        );
    }

    #[test]
    fn resolves_build_dist_cli_and_environment_overrides() {
        let temp = tempfile::tempdir().expect("temporary directory");
        assert_eq!(
            dist_directory_for_build(temp.path(), &["--dist", "build-output"]).expect("CLI dist"),
            temp.path().join("build-output")
        );
        assert_eq!(
            dist_directory_for_build(temp.path(), &["--dist=cli-output"]).expect("CLI dist equals"),
            temp.path().join("cli-output")
        );
        assert_eq!(
            dist_directory_for_build(temp.path(), &["-d", "/tmp/absolute-output"])
                .expect("absolute CLI dist"),
            Path::new("/tmp/absolute-output")
        );
    }

    #[test]
    fn clears_previous_manifest_and_validates_new_manifest() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let dist = temp.path();
        let manifest_path = dist.join(serenade_view::TRUNK_ASSET_MANIFEST_FILE);
        std::fs::write(&manifest_path, "{}").expect("old manifest");
        clear_trunk_asset_manifest(dist).expect("clear old manifest");
        assert!(!manifest_path.exists());
        assert!(validate_trunk_asset_manifest(dist).is_err());
    }

    #[test]
    fn refuses_to_remove_a_directory_at_manifest_path() {
        let temp = tempfile::tempdir().expect("temporary directory");
        std::fs::create_dir(temp.path().join(serenade_view::TRUNK_ASSET_MANIFEST_FILE))
            .expect("manifest directory");
        assert!(clear_trunk_asset_manifest(temp.path()).is_err());
    }
}
