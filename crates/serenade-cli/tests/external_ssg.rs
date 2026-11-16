//! External SSG command workflow.

#![cfg(unix)]

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    thread,
};

use serenade_view::{TRUNK_ASSET_MANIFEST_FILE, TrunkAssetManifest};

fn executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write executable");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .expect("set executable permissions");
}

#[test]
fn external_ssg_forwards_profile_and_environment_to_trunk_and_cargo() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let app = temp.path().join("consumer").join("site");
    let tools = temp.path().join("home/tools/trunk-itc-dev");
    let bin = temp.path().join("bin");
    fs::create_dir_all(&app).expect("create application");
    fs::create_dir_all(tools.join("bin")).expect("create Trunk tools");
    fs::create_dir_all(&bin).expect("create executable directory");
    fs::write(app.join("Cargo.toml"), "[package]\nname = \"site\"\n")
        .expect("write application manifest");
    fs::write(
        tools.join(".serenade-trunk-sha256"),
        format!("{}\n", "0".repeat(64)),
    )
    .expect("write Trunk checksum marker");
    fs::write(
        tools.join(".serenade-trunk-provenance"),
        format!("commit {}\n", "0".repeat(40)),
    )
    .expect("write Trunk provenance marker");

    executable(
        &tools.join("bin/trunk"),
        r#"#!/bin/sh
set -eu
test "$1" = build
mkdir -p dist
printf '%s' "$*" > dist/trunk-args.txt
printf '<html><head></head><body>shell</body></html>' > dist/index.html
printf '{"public_url":"/","html":"index.html","assets":[],"wasm":[],"document_assets":[]}' > dist/trunk-assets.json
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

/// Builds the real `demo-ssg` application through `serenade-ssg --manifest-path`
/// with the pinned Trunk, its own `prerender` binary and Cargo, then serves the
/// output and checks in a headless browser that every route hydrates.
#[test]
#[ignore = "needs the wasm32 target, Serenade's Trunk and Chrome; run with --ignored"]
fn external_ssg_prerenders_and_hydrates_the_demo_ssg_application() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let app = repository
        .join("examples/demo-monorepo/apps/demo-ssg")
        .canonicalize()
        .expect("demo-ssg application");
    let dist = app.join("dist");
    let outside = tempfile::tempdir().expect("directory outside the repository");
    let chrome = chrome();

    for flags in [&[][..], &["--release"][..]] {
        if dist.exists() {
            fs::remove_dir_all(&dist).expect("remove previous output");
        }
        let output = Command::new(env!("CARGO_BIN_EXE_serenade-ssg"))
            .arg("--manifest-path")
            .arg(app.join("Cargo.toml"))
            .args(flags)
            .current_dir(outside.path())
            .output()
            .expect("run SSG command");
        assert!(
            output.status.success(),
            "SSG {flags:?} failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let manifest = TrunkAssetManifest::from_json(
            &fs::read(dist.join(TRUNK_ASSET_MANIFEST_FILE)).expect("Trunk asset manifest"),
        )
        .expect("valid Trunk asset manifest");
        manifest
            .validate_output(&dist)
            .expect("hashed outputs match the manifest");
        let [bundle] = manifest.wasm.as_slice() else {
            panic!("expected one WASM bundle: {:?}", manifest.wasm);
        };
        let stylesheet = manifest
            .assets
            .iter()
            .find(|asset| asset.role == "stylesheet")
            .expect("hashed stylesheet");
        assert_ne!(manifest.document_assets.len(), 0);
        let referenced = [&stylesheet.url, &bundle.js_url, &bundle.wasm_url];

        let server = StaticServer::start(&dist);
        for (route, heading) in [("/", "Hello from Serenade"), ("/about/", "About Serenade")] {
            let file = dist.join(route.trim_start_matches('/')).join("index.html");
            let page = fs::read_to_string(&file).expect("prerendered route");
            let head = page.split_once("</head>").expect("prerendered head").0;
            for url in referenced {
                assert!(
                    head.contains(url.as_str()),
                    "{route} does not reference Trunk output {url}: {head}"
                );
            }
            assert!(head.contains("TrunkApplicationStarted"), "{head}");
            assert!(page.contains(&format!("<h1>{heading}</h1>")), "{page}");
            assert!(!page.contains("data-hydrated"), "{page}");

            let dom = server.render_in_browser(&chrome, route);
            assert!(
                dom.contains("data-hydrated"),
                "{route} did not hydrate with {flags:?}: {dom}"
            );
            assert!(dom.contains(&format!("<h1>{heading}</h1>")), "{dom}");
        }
        let delivered = server.served();
        for url in referenced {
            assert!(
                delivered.contains(url),
                "{url} was not delivered to the browser; served {delivered:?}"
            );
        }
    }
}

fn chrome() -> PathBuf {
    if let Some(path) = std::env::var_os("SERENADE_TEST_CHROME") {
        return PathBuf::from(path);
    }
    [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
    ]
    .into_iter()
    .find_map(|name| {
        std::env::split_paths(&std::env::var_os("PATH")?)
            .map(|directory| directory.join(name))
            .find(|path| path.is_file())
    })
    .expect("Chrome or Chromium on PATH, or SERENADE_TEST_CHROME")
}

/// Minimal HTTP server for a static output directory, recording what it served.
struct StaticServer {
    origin: String,
    served: Arc<Mutex<Vec<String>>>,
}

impl StaticServer {
    fn start(root: &Path) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind static server");
        let origin = format!("http://{}", listener.local_addr().expect("server address"));
        let served = Arc::new(Mutex::new(Vec::new()));
        let (root, log) = (root.to_path_buf(), Arc::clone(&served));
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (root, log) = (root.clone(), Arc::clone(&log));
                thread::spawn(move || serve(stream, &root, &log));
            }
        });
        Self { origin, served }
    }

    fn render_in_browser(&self, chrome: &Path, route: &str) -> String {
        let profile = tempfile::tempdir().expect("browser profile");
        let output = Command::new(chrome)
            .args([
                "--headless=new",
                "--no-sandbox",
                "--disable-gpu",
                "--no-first-run",
                "--virtual-time-budget=20000",
                "--dump-dom",
            ])
            .arg(format!("--user-data-dir={}", profile.path().display()))
            .arg(format!("{}{route}", self.origin))
            .output()
            .expect("run headless browser");
        assert!(
            output.status.success(),
            "browser failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("UTF-8 DOM")
    }

    fn served(&self) -> Vec<String> {
        self.served.lock().expect("served log").clone()
    }
}

fn serve(mut stream: TcpStream, root: &Path, served: &Mutex<Vec<String>>) {
    let mut reader = BufReader::new(&stream);
    let mut request = String::new();
    if reader.read_line(&mut request).is_err() {
        return;
    }
    let mut header = String::new();
    while reader.read_line(&mut header).is_ok_and(|read| read > 2) {
        header.clear();
    }
    let path = request
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .split(['?', '#'])
        .next()
        .unwrap_or("/")
        .to_owned();
    let mut file = root.join(path.trim_start_matches('/'));
    if file.is_dir() {
        file = file.join("index.html");
    }
    let body = (!path.contains(".."))
        .then(|| fs::read(&file).ok())
        .flatten();
    let Some(body) = body else {
        _ = stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    };
    served.lock().expect("served log").push(path);
    let content_type = match file.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css",
        Some("js") => "text/javascript",
        Some("wasm") => "application/wasm",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    };
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    _ = stream
        .write_all(header.as_bytes())
        .and_then(|()| stream.write_all(&body));
}
