//! Post-processes static HTML output produced by frontend and SSG builds.

use std::path::Path;

use anyhow::{Context, bail};
use serde::Deserialize;
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
}
