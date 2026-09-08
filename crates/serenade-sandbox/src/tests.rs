use super::*;
use serde::Deserialize;
use std::ffi::OsString;
use std::path::PathBuf;

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

#[tokio::test]
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
