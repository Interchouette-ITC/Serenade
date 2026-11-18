use std::{
    env,
    fs::File,
    path::{Path, PathBuf},
    time::SystemTime,
};

use rangular_build::{Config, HeadComponentAdapter, build};

/// Compiles the CMS Hello document, head, and body templates with Rangular's
/// build-script API. Generated views land in `OUT_DIR/rangular/` as
/// `document_view.rs`, `head_view.rs`, and `body_view.rs`.
pub fn compile(template_dir: impl AsRef<Path>) {
    let template_dir = template_dir.as_ref();
    let css_output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("cms_hello.css");
    build(
        Config::new(template_dir, &css_output)
            .document(template_dir.join("document.html"))
            .head(template_dir.join("head.html"))
            .body(template_dir.join("body.html"))
            .head_adapter(HeadComponentAdapter::new(
                "serenade_view::ManagedTitle",
                "serenade_view::ManagedMeta",
                "serenade_view::ManagedLink",
            )),
    );
    // rangular-build 1.1.0 rewrites the stylesheet on every run and also watches it with
    // `rerun-if-changed`, which would leave the package permanently dirty. Backdate it so only
    // template edits trigger a rerun.
    File::options()
        .write(true)
        .open(&css_output)
        .and_then(|file| file.set_modified(SystemTime::UNIX_EPOCH))
        .expect("backdate generated Rangular stylesheet");
}
