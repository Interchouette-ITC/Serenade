use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub fn compile(template_dir: impl AsRef<Path>) {
    let template_dir = template_dir.as_ref();
    for template in ["document.html", "head.html", "body.html"] {
        println!(
            "cargo:rerun-if-changed={}",
            template_dir.join(template).display()
        );
    }

    let document_source = fs::read_to_string(template_dir.join("document.html"))
        .expect("read Rangular CMS document template");
    let head_source = fs::read_to_string(template_dir.join("head.html"))
        .expect("read Rangular CMS head template");
    let body_source = fs::read_to_string(template_dir.join("body.html"))
        .expect("read Rangular CMS body template");
    let adapter = rangular_aot::HeadComponentAdapter::new(
        "serenade_view::ManagedTitle",
        "serenade_view::ManagedMeta",
        "serenade_view::ManagedLink",
    );
    let document = rangular_aot::compile_named(&document_source, "document.html", "cms_document");
    let head = rangular_aot::compile_head_named_with_adapter(
        &head_source,
        "head.html",
        "cms_head",
        &adapter,
    );
    let body = rangular_aot::compile_named(&body_source, "body.html", "cms_body");

    assert!(
        document.ok(),
        "document compilation failed: {:?}",
        document.issues
    );
    assert!(head.ok(), "head compilation failed: {:?}", head.issues);
    assert!(body.ok(), "body compilation failed: {:?}", body.issues);

    let mut generated = document.code;
    generated.push_str(&head.code);
    generated.push_str(&body.code);
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("cms_hello.rs");
    fs::write(output, generated).expect("write AOT-generated CMS templates");
}
