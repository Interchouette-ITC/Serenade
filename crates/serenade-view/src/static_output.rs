//! Static document output for route-oriented prerendering.

use std::{
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use crate::{RenderedDocument, ViewError};

/// Environment variable set by `serenade-ssg` for the static output directory.
pub const STATIC_OUTPUT_DIR_ENV: &str = "SERENADE_STATIC_OUTPUT_DIR";

/// Writes complete documents to route-derived `index.html` files.
///
/// Applications enumerate routes and provide page data; this writer owns the
/// filesystem mapping from each route to its static output path. This
/// server-side API is omitted from `wasm32` builds.
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
        Ok(self.index_path(&route_segments(route)?))
    }

    /// Writes a rendered document at the static path for `route`.
    ///
    /// Route directories below the output directory are created one level at a
    /// time and must be real directories: a symbolic link there is rejected, so
    /// a document is never written outside the output directory. The document
    /// is written to a uniquely named temporary file that then replaces
    /// `index.html` (on Unix and Windows), so an existing symbolic link is
    /// replaced instead of written through and concurrent writes of the same
    /// route do not interfere; the last completed write wins.
    ///
    /// # Errors
    ///
    /// Returns an error when the route is unsafe, a route directory is a
    /// symbolic link or not a directory, or the document cannot be written.
    pub fn write(&self, route: &str, document: RenderedDocument) -> Result<PathBuf, ViewError> {
        let segments = route_segments(route)?;
        let output_path = self.index_path(&segments);
        let mut directory = fs::create_dir_all(&self.output_directory)
            .and_then(|()| self.output_directory.canonicalize())
            .map_err(|error| directory_error(&self.output_directory, &error))?;
        for segment in segments {
            directory.push(segment);
            create_real_directory(&directory)?;
        }
        replace_file(&directory, document.into_static_output().as_bytes()).map_err(|error| {
            ViewError::msg(format!(
                "failed to write static document {}: {error}",
                output_path.display()
            ))
        })?;
        Ok(output_path)
    }
}

impl StaticDocumentWriter {
    fn index_path(&self, segments: &[&str]) -> PathBuf {
        let mut output_path = self.output_directory.clone();
        output_path.extend(segments);
        output_path.push(INDEX_FILE);
        output_path
    }
}

const INDEX_FILE: &str = "index.html";

fn route_segments(route: &str) -> Result<Vec<&str>, ViewError> {
    if !route.starts_with('/') || route.contains(['?', '#', '\\']) || route.contains("//") {
        return Err(ViewError::msg(format!(
            "static output route must be a normalized absolute path: `{route}`"
        )));
    }
    let relative = route.trim_start_matches('/');
    let relative = relative.strip_suffix('/').unwrap_or(relative);
    if relative.is_empty() {
        return Ok(Vec::new());
    }
    relative
        .split('/')
        .map(|segment| {
            let decoded_dots = segment.to_ascii_lowercase().replace("%2e", ".");
            if segment.is_empty()
                || matches!(decoded_dots.as_str(), "." | "..")
                || decoded_dots.contains("%2f")
                || decoded_dots.contains("%5c")
                || segment.contains(':')
                || segment.chars().any(char::is_control)
            {
                Err(ViewError::msg(format!(
                    "unsafe static output route segment in `{route}`"
                )))
            } else {
                Ok(segment)
            }
        })
        .collect()
}

/// Creates `path` if it is missing; an existing entry must be a real directory.
/// Concurrent writers may create the same directory, so `AlreadyExists` is
/// re-checked instead of treated as a failure.
fn create_real_directory(path: &Path) -> Result<(), ViewError> {
    match fs::create_dir(path) {
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            match fs::symlink_metadata(path) {
                Ok(metadata) if metadata.file_type().is_symlink() => Err(ViewError::msg(format!(
                    "static output directory {} is a symbolic link; refusing to write through it",
                    path.display()
                ))),
                Ok(metadata) if metadata.is_dir() => Ok(()),
                _ => Err(directory_error(
                    path,
                    &std::io::Error::other("not a directory"),
                )),
            }
        }
        result => result.map_err(|error| directory_error(path, &error)),
    }
}

fn directory_error(path: &Path, error: &std::io::Error) -> ViewError {
    ViewError::msg(format!(
        "failed to create static output directory {}: {error}",
        path.display()
    ))
}

/// Writes `contents` to a uniquely named temporary file in `directory`, then
/// replaces `index.html` with it. Each write gets its own exclusively created
/// temporary file, so concurrent writers never share one, and `persist` replaces
/// an existing destination (including a symbolic link, which is not followed)
/// on Unix and Windows. A failed write removes its temporary file.
fn replace_file(directory: &Path, contents: &[u8]) -> std::io::Result<()> {
    let destination = directory.join(INDEX_FILE);
    tempfile::Builder::new()
        .prefix(".index.html.")
        .suffix(".serenade-tmp")
        .tempfile_in(directory)
        .and_then(|mut file| file.write_all(contents).map(|()| file))
        .and_then(|file| {
            file.persist(&destination)
                .map(drop)
                .or_else(|error| retry_replacement(error, &destination))
        })
}

/// On Windows, `persist` uses `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`, which
/// replaces an existing `index.html` but refuses a read-only one. Clear the
/// read-only flag on a regular file (never through a symbolic link) and retry
/// once, so reruns replace previous output as they do on Unix.
#[cfg(windows)]
// Windows-only: clears the read-only attribute; the Unix world-writable concern does not apply.
#[allow(clippy::permissions_set_readonly_false)]
fn retry_replacement(error: tempfile::PersistError, destination: &Path) -> std::io::Result<()> {
    let tempfile::PersistError { error, file } = error;
    match fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.is_file() && metadata.permissions().readonly() => {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(false);
            fs::set_permissions(destination, permissions)?;
            file.persist(destination)
                .map(drop)
                .map_err(|error| error.error)
        }
        _ => Err(error),
    }
}

/// Unix `rename` already replaces any existing destination entry.
#[cfg(not(windows))]
fn retry_replacement(error: tempfile::PersistError, _destination: &Path) -> std::io::Result<()> {
    Err(error.error)
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
    fn rejects_route_entries_that_are_files_and_unsafe_routes() {
        let temp = tempfile::tempdir().expect("temporary directory");
        fs::write(temp.path().join("about"), "not a directory").expect("blocking file");
        let writer = StaticDocumentWriter::new(temp.path());
        let error = writer
            .write("/about/team", document())
            .expect_err("a file where a route directory belongs must fail")
            .to_string();
        assert!(error.contains("failed to create static output directory"));
        assert!(error.contains("not a directory"));
        assert!(writer.write("/../secret", document()).is_err());

        let too_long = format!("/{}", "a".repeat(4096));
        let error = writer
            .write(&too_long, document())
            .expect_err("an uncreatable route directory must fail")
            .to_string();
        assert!(error.contains("failed to create static output directory"));
    }

    fn entries(directory: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(directory)
            .expect("read directory")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn rewrites_existing_documents_without_leaving_temporary_files() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let writer = StaticDocumentWriter::new(temp.path());
        fs::write(temp.path().join(INDEX_FILE), "old").expect("existing document");
        writer
            .write("/", document())
            .expect("rewrite root document");
        assert!(
            fs::read_to_string(temp.path().join(INDEX_FILE))
                .expect("read output")
                .contains("About page")
        );
        assert_eq!(entries(temp.path()), [INDEX_FILE]);
    }

    #[test]
    fn rerun_replaces_a_read_only_existing_document() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let existing = temp.path().join(INDEX_FILE);
        fs::write(&existing, "old").expect("existing document");
        let mut permissions = fs::metadata(&existing).expect("metadata").permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&existing, permissions).expect("mark read-only");
        StaticDocumentWriter::new(temp.path())
            .write("/", document())
            .expect("rerun over a read-only document");
        assert!(
            fs::read_to_string(&existing)
                .expect("read output")
                .contains("About page")
        );
        assert_eq!(entries(temp.path()), [INDEX_FILE]);
    }

    #[test]
    fn concurrent_writes_of_the_same_route_all_succeed() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let writer = StaticDocumentWriter::new(temp.path());
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    for _ in 0..25 {
                        writer
                            .write("/docs/guide/", document())
                            .expect("concurrent write");
                    }
                });
            }
        });
        let route = temp.path().join("docs/guide");
        assert_eq!(entries(&route), [INDEX_FILE]);
        assert!(
            fs::read_to_string(route.join(INDEX_FILE))
                .expect("read output")
                .contains("About page")
        );
    }

    #[test]
    #[cfg(unix)]
    fn never_writes_through_symbolic_links_out_of_the_output_directory() {
        use std::os::unix::fs::symlink;

        let outside = tempfile::tempdir().expect("outside directory");
        let temp = tempfile::tempdir().expect("temporary directory");
        let writer = StaticDocumentWriter::new(temp.path());

        symlink(outside.path(), temp.path().join("about")).expect("directory symlink");
        let error = writer
            .write("/about/team", document())
            .expect_err("symlinked route directory must be rejected")
            .to_string();
        assert!(error.contains("is a symbolic link"));
        assert!(!outside.path().join("team").exists());

        let victim = outside.path().join("victim.html");
        fs::write(&victim, "untouched").expect("outside file");
        symlink(&victim, temp.path().join(INDEX_FILE)).expect("file symlink");
        writer
            .write("/", document())
            .expect("replace symlinked document");
        assert_eq!(fs::read_to_string(&victim).expect("victim"), "untouched");
        let written = temp.path().join(INDEX_FILE);
        assert!(
            !fs::symlink_metadata(&written)
                .expect("metadata")
                .file_type()
                .is_symlink()
        );
        assert!(
            fs::read_to_string(written)
                .expect("output")
                .contains("About page")
        );

        let linked_root = outside.path().join("linked-output");
        symlink(temp.path(), &linked_root).expect("output directory symlink");
        StaticDocumentWriter::new(&linked_root)
            .write("/", document())
            .expect("a symlinked output directory itself is the configured root");
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
