//! Static document output for route-oriented prerendering.

use std::{ffi::OsString, fs, path::PathBuf};

use crate::{RenderedDocument, ViewError};

/// Environment variable set by `serenade-ssg` for the static output directory.
pub const STATIC_OUTPUT_DIR_ENV: &str = "SERENADE_STATIC_OUTPUT_DIR";

/// Writes complete documents to route-derived `index.html` files.
///
/// Applications enumerate routes and provide page data; this writer owns the
/// filesystem mapping from each route to its static output path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StaticDocumentWriter {
    output_directory: PathBuf,
}

impl StaticDocumentWriter {
    /// Creates a writer rooted at `output_directory`.
    #[must_use]
    pub fn new(output_directory: impl Into<PathBuf>) -> Self {
        Self {
            output_directory: output_directory.into(),
        }
    }

    /// Creates a writer using the output directory supplied by `serenade-ssg`.
    ///
    /// # Errors
    ///
    /// Returns an error when the environment variable is missing or empty.
    pub fn from_env() -> Result<Self, ViewError> {
        Self::from_environment(std::env::var_os(STATIC_OUTPUT_DIR_ENV))
    }

    fn from_environment(output_directory: Option<OsString>) -> Result<Self, ViewError> {
        let output_directory = output_directory
            .filter(|directory| !directory.is_empty())
            .ok_or_else(|| {
                ViewError::msg(format!(
                    "environment variable {STATIC_OUTPUT_DIR_ENV} is not set"
                ))
            })?;
        Ok(Self::new(output_directory))
    }

    /// Resolves a normalized URL route to its static `index.html` path.
    ///
    /// `/` maps to `<output>/index.html`; `/about` and `/about/` map to
    /// `<output>/about/index.html`. Query strings, fragments, and unsafe path
    /// segments are rejected.
    ///
    /// # Errors
    ///
    /// Returns an error when `route` is not a safe absolute URL path.
    pub fn output_path(&self, route: &str) -> Result<PathBuf, ViewError> {
        if !route.starts_with('/') || route.contains(['?', '#', '\\']) || route.contains("//") {
            return Err(ViewError::msg(format!(
                "static output route must be a normalized absolute path: `{route}`"
            )));
        }

        let relative = route
            .strip_prefix('/')
            .and_then(|route| route.strip_suffix('/').or(Some(route)))
            .unwrap_or_default();
        let mut output_path = self.output_directory.clone();

        if !relative.is_empty() {
            for segment in relative.split('/') {
                let decoded_dots = segment.to_ascii_lowercase().replace("%2e", ".");
                if segment.is_empty()
                    || matches!(decoded_dots.as_str(), "." | "..")
                    || decoded_dots.contains("%2f")
                    || decoded_dots.contains("%5c")
                    || segment.contains(':')
                    || segment.chars().any(char::is_control)
                {
                    return Err(ViewError::msg(format!(
                        "unsafe static output route segment in `{route}`"
                    )));
                }
                output_path.push(segment);
            }
        }

        output_path.push("index.html");
        Ok(output_path)
    }

    /// Writes a rendered document at the static path for `route`.
    ///
    /// # Errors
    ///
    /// Returns an error when the route is unsafe or the output directory or
    /// document cannot be written.
    pub fn write(&self, route: &str, document: RenderedDocument) -> Result<PathBuf, ViewError> {
        let output_path = self.output_path(route)?;
        if let Some(parent) = output_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| {
                ViewError::msg(format!(
                    "failed to create static output directory {}: {error}",
                    parent.display()
                ))
            })?;
        }
        fs::write(&output_path, document.into_static_output()).map_err(|error| {
            ViewError::msg(format!(
                "failed to write static document {}: {error}",
                output_path.display()
            ))
        })?;
        Ok(output_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, DocumentParts, DocumentTemplate};

    struct TestTemplate;

    impl DocumentTemplate for TestTemplate {
        type Head = &'static str;
        type Body = &'static str;

        fn render(
            &self,
            parts: DocumentParts<Self::Head, Self::Body>,
        ) -> Result<String, ViewError> {
            Ok(format!(
                "<!doctype html><html><head>{}</head><body>{}</body></html>",
                parts.head, parts.body
            ))
        }
    }

    fn document() -> RenderedDocument {
        Document::new("<title>About</title>", "<main>About page</main>")
            .render(&TestTemplate)
            .expect("render document")
    }

    #[test]
    fn maps_routes_to_static_index_documents() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let writer = StaticDocumentWriter::new(temp.path());

        assert_eq!(
            writer.output_path("/").expect("root route"),
            temp.path().join("index.html")
        );
        assert_eq!(
            writer.output_path("/about").expect("about route"),
            temp.path().join("about/index.html")
        );
        assert_eq!(
            writer.output_path("/about/").expect("trailing slash route"),
            temp.path().join("about/index.html")
        );
    }

    #[test]
    fn reads_output_directory_from_environment_value() {
        assert_eq!(
            StaticDocumentWriter::from_environment(Some(OsString::from("dist")))
                .expect("configured output directory"),
            StaticDocumentWriter::new("dist")
        );
        assert!(StaticDocumentWriter::from_environment(None).is_err());
        assert!(StaticDocumentWriter::from_environment(Some(OsString::new())).is_err());

        let _from_env = StaticDocumentWriter::from_env();
    }

    #[test]
    fn writes_complete_documents_and_creates_route_directories() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let writer = StaticDocumentWriter::new(temp.path());
        let output_path = writer.write("/about/", document()).expect("write document");

        assert_eq!(output_path, temp.path().join("about/index.html"));
        assert_eq!(
            fs::read_to_string(output_path).expect("read output"),
            "<!doctype html><html><head><title>About</title></head><body><main>About page</main></body></html>"
        );
    }

    #[test]
    fn reports_static_output_filesystem_errors() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let blocked_directory = temp.path().join("blocked");
        fs::write(&blocked_directory, "not a directory").expect("create blocking file");
        let error = StaticDocumentWriter::new(blocked_directory)
            .write("/about", document())
            .expect_err("creating a route directory beneath a file must fail")
            .to_string();
        assert!(error.contains("failed to create static output directory"));

        let temp = tempfile::tempdir().expect("temporary directory");
        fs::create_dir_all(temp.path().join("about/index.html"))
            .expect("create directory at document file path");
        let error = StaticDocumentWriter::new(temp.path())
            .write("/about", document())
            .expect_err("writing over a directory must fail")
            .to_string();
        assert!(error.contains("failed to write static document"));
    }

    #[test]
    fn rejects_non_normalized_and_unsafe_routes() {
        let writer = StaticDocumentWriter::new("dist");
        for route in [
            "about",
            "/about?tab=details",
            "/about#details",
            "/about//team",
            "/../secret",
            "/about/%2e%2e/secret",
            "/about/%2Fsecret",
            "/about\\secret",
            "/C:/secret",
        ] {
            assert!(
                writer.output_path(route).is_err(),
                "route should be rejected: {route}"
            );
        }
    }
}
