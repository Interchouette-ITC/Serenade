use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serenade_view::{
    Document, DocumentParts, DocumentTemplate, STATIC_OUTPUT_DIR_ENV, StaticDocumentWriter,
    TrunkAssetManifest, ViewError,
};

const CONTENT_MARKER: &str = "<!--SERENADE_SSG_CONTENT-->";
const ROUTES: [(&str, &str); 2] = [("/", "home"), ("/about", "about")];

/// Trunk's generated HTML owns the document structure and hashed asset tags;
/// each route's prerendered body fills its single content slot.
struct TrunkShell {
    html: String,
}

impl TrunkShell {
    fn new(html: String) -> anyhow::Result<Self> {
        if html.matches(CONTENT_MARKER).count() != 1 {
            bail!("Trunk output must contain exactly one {CONTENT_MARKER} marker");
        }
        Ok(Self { html })
    }
}

impl DocumentTemplate for TrunkShell {
    type Head = ();
    type Body = String;

    fn render(&self, parts: DocumentParts<(), String>) -> Result<String, ViewError> {
        Ok(self.html.replacen(CONTENT_MARKER, &parts.body, 1))
    }
}

fn main() -> anyhow::Result<()> {
    let output = std::env::var_os(STATIC_OUTPUT_DIR_ENV)
        .filter(|directory| !directory.is_empty())
        .map(PathBuf::from)
        .with_context(|| format!("{STATIC_OUTPUT_DIR_ENV} is not set; run through serenade-ssg"))?;
    let manifest = TrunkAssetManifest::read_from_env()?;
    prerender(&output, &manifest)?;
    println!("Prerendered / and /about/ to {}", output.display());
    Ok(())
}

fn prerender(output: &Path, manifest: &TrunkAssetManifest) -> anyhow::Result<()> {
    manifest.validate_output(output).with_context(|| {
        format!(
            "Trunk output {} does not match its manifest",
            output.display()
        )
    })?;
    let shell_path = output.join(&manifest.html);
    let shell = TrunkShell::new(
        fs::read_to_string(&shell_path)
            .with_context(|| format!("read Trunk output {}", shell_path.display()))?,
    )?;
    let writer = StaticDocumentWriter::new(output);
    for (route, page) in ROUTES {
        let rendered = Document::new((), serenade_demo_ssg::render_page(page)).render(&shell)?;
        writer.write(route, rendered)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output_with_shell(shell: &str) -> (PathBuf, TrunkAssetManifest) {
        let output = std::env::temp_dir().join(format!(
            "serenade-demo-ssg-prerender-{}-{}",
            std::process::id(),
            shell.len()
        ));
        fs::create_dir_all(&output).expect("output directory");
        fs::write(output.join("shell.html"), shell).expect("shell");
        let manifest = TrunkAssetManifest::from_json(
            br#"{"version":2,"public_url":"/","html":"shell.html","assets":[],"wasm":[]}"#,
        )
        .expect("valid manifest");
        (output, manifest)
    }

    #[test]
    fn prerenders_routes_into_the_supplied_output_from_the_manifest_shell() {
        let (output, manifest) = output_with_shell(
            "<html><head><script src=/app-1a2b.js></script></head><body><!--SERENADE_SSG_CONTENT--></body></html>",
        );
        prerender(&output, &manifest).expect("prerender");
        let home = fs::read_to_string(output.join("index.html")).expect("home route");
        let about = fs::read_to_string(output.join("about/index.html")).expect("about route");
        for page in [&home, &about] {
            assert!(page.contains("/app-1a2b.js"), "{page}");
            assert!(!page.contains(CONTENT_MARKER), "{page}");
        }
        assert!(home.contains("Hello from Serenade"), "{home}");
        assert!(about.contains("About Serenade"), "{about}");
        fs::remove_dir_all(output).expect("cleanup");
    }

    #[test]
    fn rejects_a_shell_without_exactly_one_content_marker_or_a_missing_shell() {
        let (output, manifest) = output_with_shell("<html><body></body></html>");
        assert!(prerender(&output, &manifest).is_err());
        assert!(prerender(&output.join("missing"), &manifest).is_err());
        fs::remove_dir_all(output).expect("cleanup");
    }
}
