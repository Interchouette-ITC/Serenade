use super::*;
use serde::Deserialize;
use std::ffi::OsString;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct TinyJson {
    ok: bool,
}

#[test]
fn version_is_nonempty() {
    assert_ne!(version(), "");
}

#[test]
fn cache_root_from_env_override() {
    let root = wasmer_cache_root_from(Some(OsString::from("/tmp/serenade-wasmer-cache")));
    assert_eq!(root, PathBuf::from("/tmp/serenade-wasmer-cache"));
}

#[test]
fn cache_root_default_is_relative_wasmer() {
    let root = wasmer_cache_root_from(None);
    assert_eq!(root, PathBuf::from(".wasmer"));
}

#[test]
fn wasmer_cache_root_returns_path() {
    let root = wasmer_cache_root();
    assert!(!root.as_os_str().is_empty());
}

#[test]
fn decode_json_success() {
    let output = GuestOutput {
        stdout: br#"{"ok":true}"#.to_vec(),
        stderr: Vec::new(),
        success: true,
    };
    let parsed: TinyJson = decode_json(&output).expect("json");
    assert_eq!(parsed, TinyJson { ok: true });
}

#[test]
fn decode_json_rejects_failed_guest() {
    let output = GuestOutput {
        stdout: Vec::new(),
        stderr: b"boom".to_vec(),
        success: false,
    };
    assert!(decode_json::<TinyJson>(&output).is_err());
}

#[test]
fn decode_json_rejects_empty_stdout_with_stderr() {
    let output = GuestOutput {
        stdout: Vec::new(),
        stderr: b"hint".to_vec(),
        success: true,
    };
    let err = decode_json::<TinyJson>(&output).expect_err("empty stdout");
    assert!(err.to_string().contains("no stdout"));
}

#[test]
fn decode_json_rejects_invalid_json() {
    let output = GuestOutput {
        stdout: b"not-json".to_vec(),
        stderr: b"trace".to_vec(),
        success: true,
    };
    let err = decode_json::<TinyJson>(&output).expect_err("bad json");
    assert!(err.to_string().contains("parse guest JSON"));
}

#[tokio::test(flavor = "multi_thread")]
async fn run_module_echoes_stdin_json() {
    let wasm = include_bytes!("../fixtures/wasi-echo.wasm");
    let cache = tempfile::tempdir().expect("temp cache");
    let stdin = br#"{"ok":true}"#;
    let output = run_module("wasi-echo", wasm, stdin, cache.path())
        .await
        .expect("run module");
    assert!(
        output.success,
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: TinyJson = decode_json(&output).expect("decode");
    assert_eq!(parsed, TinyJson { ok: true });
}

#[tokio::test(flavor = "multi_thread")]
async fn load_webc_from_cache_hit() {
    let cache = tempfile::tempdir().expect("temp cache");
    let downloads = cache.path().join("downloads");
    std::fs::create_dir_all(&downloads).expect("downloads");
    let payload = vec![0xAB; 2048];
    std::fs::write(downloads.join("cached.webc"), &payload).expect("write cache");
    let body = load_webc_from(cache.path(), "http://127.0.0.1:9/unused", "cached.webc")
        .await
        .expect("cache hit");
    assert_eq!(body.as_ref(), payload.as_slice());
}

#[tokio::test(flavor = "multi_thread")]
async fn load_webc_from_downloads_and_rejects_small_payload() {
    let cache = tempfile::tempdir().expect("temp cache");
    let (url, server) = spawn_http_server(b"tiny".to_vec(), 200).await;
    let err = load_webc_from(cache.path(), &url, "small.webc")
        .await
        .expect_err("too small");
    server.abort();
    assert!(err.to_string().contains("too small"));
}

#[tokio::test(flavor = "multi_thread")]
async fn load_webc_from_maps_http_error() {
    let cache = tempfile::tempdir().expect("temp cache");
    let (url, server) = spawn_http_server(b"nope".to_vec(), 404).await;
    let err = load_webc_from(cache.path(), &url, "missing.webc")
        .await
        .expect_err("http error");
    server.abort();
    assert!(err.to_string().contains("failed with HTTP"));
}

#[tokio::test(flavor = "multi_thread")]
async fn load_webc_from_downloads_large_payload() {
    let cache = tempfile::tempdir().expect("temp cache");
    let payload = vec![0xCD; 2048];
    let (url, server) = spawn_http_server(payload.clone(), 200).await;
    let body = load_webc_from(cache.path(), &url, "dl.webc")
        .await
        .expect("download");
    server.abort();
    assert_eq!(body.as_ref(), payload.as_slice());
    assert!(cache.path().join("downloads").join("dl.webc").is_file());
}

#[tokio::test(flavor = "multi_thread")]
async fn run_package_echoes_stdin_json() {
    let webc = build_echo_webc();
    let cache = tempfile::tempdir().expect("temp cache");
    let downloads = cache.path().join("downloads");
    std::fs::create_dir_all(&downloads).expect("downloads");
    std::fs::write(downloads.join("echo.webc"), &webc).expect("write webc");

    let stdin = br#"{"ok":true}"#;
    let output = run_package(PackageRun {
        stdin_bytes: stdin,
        cache_root: cache.path(),
        package_url: "http://127.0.0.1:9/unused",
        cache_name: "echo.webc",
        command: "echo",
        args: Vec::new(),
    })
    .await
    .expect("run package");
    assert!(
        output.success,
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: TinyJson = decode_json(&output).expect("decode");
    assert_eq!(parsed, TinyJson { ok: true });
}

fn build_echo_webc() -> bytes::Bytes {
    let dir = tempfile::tempdir().expect("webc dir");
    let wasm = include_bytes!("../fixtures/wasi-echo.wasm");
    std::fs::write(dir.path().join("echo.wasm"), wasm).expect("write wasm");
    let manifest = r#"
[package]
name = "serenade/echo"
version = "0.0.1"
description = "echo fixture for tests"

[[module]]
name = "echo"
source = "echo.wasm"
abi = "wasi"

[[command]]
name = "echo"
module = "echo"
runner = "wasi"
"#;
    std::fs::write(dir.path().join("wasmer.toml"), manifest).expect("write manifest");
    wasmer_package::package::Package::from_manifest(dir.path().join("wasmer.toml"))
        .expect("package")
        .serialize()
        .expect("serialize webc")
}

async fn spawn_http_server(body: Vec<u8>, status: u16) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let handle = tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            return;
        };
        let mut buf = [0_u8; 1024];
        let _ = socket.read(&mut buf).await;
        let reason = if status == 200 { "OK" } else { "ERR" };
        let header = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = socket.write_all(header.as_bytes()).await;
        let _ = socket.write_all(&body).await;
    });
    (format!("http://{addr}/pkg.webc"), handle)
}
