//! External SSG command workflow.

#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

fn executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write executable");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .expect("set executable permissions");
}

#[test]
fn external_ssg_builds_validates_prerenders_and_minifies() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let app = temp.path().join("consumer").join("site");
    let tools = temp.path().join("home/tools/trunk-v0.22.1-itc.5");
    let bin = temp.path().join("bin");
    fs::create_dir_all(&app).expect("create application");
    fs::create_dir_all(tools.join("bin")).expect("create Trunk tools");
    fs::create_dir_all(&bin).expect("create executable directory");
    fs::write(app.join("Cargo.toml"), "[package]\nname = \"site\"\n")
        .expect("write application manifest");
    fs::write(tools.join(".serenade-trunk-tag"), "v0.22.1-itc.5\n")
        .expect("write Trunk version marker");

    executable(
        &tools.join("bin/trunk"),
        r#"#!/bin/sh
set -eu
test "$1" = build
mkdir -p dist
printf '%s' "$*" > dist/trunk-args.txt
printf '<html><head></head><body>shell</body></html>' > dist/index.html
printf '{"version":3,"public_url":"/","html":"index.html","assets":[],"wasm":[],"document_assets":[]}' > dist/trunk-assets.json
"#,
    );
    executable(
        &bin.join("cargo"),
        r#"#!/bin/sh
set -eu
printf '%s\n' "$PWD" "$*" "$SERENADE_STATIC_OUTPUT_DIR" "$SERENADE_TRUNK_ASSET_MANIFEST" > "$SERENADE_STATIC_OUTPUT_DIR/invocation.txt"
printf '<!DOCTYPE html><html><head><title>Page</title></head><body>  Content  </body></html>' > "$SERENADE_STATIC_OUTPUT_DIR/index.html"
"#,
    );

    let path = std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("PATH"),
    )))
    .expect("combine PATH");
    for (flags, profile) in [(&[][..], ""), (&["--release"][..], "--release ")] {
        let output = Command::new(env!("CARGO_BIN_EXE_serenade-ssg"))
            .args(["--manifest-path"])
            .arg(app.join("Cargo.toml"))
            .args(flags)
            .current_dir(temp.path())
            .env("SERENADE_HOME", temp.path().join("home"))
            .env("PATH", &path)
            .output()
            .expect("run SSG command");
        assert!(
            output.status.success(),
            "SSG failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let dist = app.join("dist");
        let invocation =
            fs::read_to_string(dist.join("invocation.txt")).expect("prerender invocation");
        let manifest = app.join("Cargo.toml");
        assert!(invocation.contains(&app.display().to_string()));
        assert!(invocation.contains(&format!(
        "run --locked --manifest-path {} {profile}--bin prerender --no-default-features --features ssr",
        manifest.display()
    )));
        assert_eq!(
            fs::read_to_string(dist.join("trunk-args.txt")).expect("Trunk arguments"),
            format!("build {profile}--features hydrate")
        );
        assert!(invocation.contains(&dist.display().to_string()));
        assert!(invocation.contains(&dist.join("trunk-assets.json").display().to_string()));
        let html = fs::read_to_string(dist.join("index.html")).expect("prerendered document");
        assert!(html.contains("<title>Page</title>"));
        assert!(html.contains("Content"));
        assert!(
            !html.contains("  Content  "),
            "HTML was not minified: {html}"
        );
    }
}

#[test]
fn external_ssg_rejects_a_workspace_manifest() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let manifest: PathBuf = temp.path().join("Cargo.toml");
    fs::write(&manifest, "[workspace]\nmembers = []\n").expect("write workspace manifest");
    let output = Command::new(env!("CARGO_BIN_EXE_serenade-ssg"))
        .arg("--manifest-path")
        .arg(&manifest)
        .output()
        .expect("run SSG command");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("workspace manifest"));
}
