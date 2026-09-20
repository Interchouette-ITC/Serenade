use std::path::PathBuf;
use std::time::Duration;

use crate::{Process, ProcessError};

#[test]
fn echo_must_run_captures_stdout() {
    let out = Process::new("echo")
        .arg("serenade")
        .must_run()
        .expect("echo");
    assert!(out.is_successful());
    assert_eq!(out.code(), Some(0));
    assert!(out.stdout_string().contains("serenade"));
    assert!(out.stdout().windows(8).any(|w| w == b"serenade"));
    assert_eq!(out.stderr(), b"");
    assert_eq!(out.stderr_string(), "");
    assert_eq!(out.status().code(), Some(0));
}

#[test]
fn false_run_is_unsuccessful_must_run_fails() {
    let out = Process::new("false").run().expect("spawn false");
    assert!(!out.is_successful());
    let err = Process::new("false").must_run().expect_err("must");
    assert_eq!(
        err,
        ProcessError::Failed {
            command: "false".into(),
            code: Some(1),
        }
    );
}

#[test]
fn timeout_kills_long_sleep() {
    let err = Process::new("sleep")
        .arg("30")
        .timeout(Duration::from_millis(100))
        .run()
        .expect_err("timeout");
    assert_eq!(
        err,
        ProcessError::TimedOut {
            command: "sleep".into(),
            timeout: Duration::from_millis(100),
        }
    );
}

#[test]
fn timeout_allows_fast_command() {
    let out = Process::new("true")
        .timeout(Duration::from_secs(5))
        .must_run()
        .expect("true within timeout");
    assert!(out.is_successful());
}

#[test]
fn stderr_string_from_command() {
    let out = Process::new("sh")
        .args(["-c", "printf '%s' 'err' 1>&2"])
        .run()
        .expect("stderr");
    assert_eq!(out.stderr_string(), "err");
    assert_eq!(out.stdout(), b"");
}

#[test]
fn cwd_and_env() {
    let dir = tempfile_dir();
    let out = Process::new("pwd").cwd(&dir).must_run().expect("pwd");
    let pwd = out.stdout_string();
    assert!(
        pwd.contains(dir.file_name().and_then(|s| s.to_str()).unwrap_or("")),
        "pwd={pwd} dir={}",
        dir.display()
    );

    let out = Process::new("sh")
        .args(["-c", "printf '%s' \"$SERENADE_PROC_TEST\""])
        .env("SERENADE_PROC_TEST", "ok")
        .must_run()
        .expect("env");
    assert_eq!(out.stdout_string(), "ok");
}

#[test]
fn clear_env_drops_inherited() {
    // SAFETY: test-only; sets a parent env var then spawns with clear_env.
    unsafe {
        std::env::set_var("SERENADE_PROC_CLEAR", "leak");
    }
    let out = Process::new("sh")
        .args(["-c", "printf '%s' \"${SERENADE_PROC_CLEAR:-}\""])
        .clear_env()
        .env("PATH", "/usr/bin:/bin")
        .must_run()
        .expect("clear");
    assert_eq!(out.stdout_string(), "");
    unsafe {
        std::env::remove_var("SERENADE_PROC_CLEAR");
    }
}

#[test]
fn empty_command_and_invalid_cwd() {
    assert_eq!(
        Process::new("").run().expect_err("empty"),
        ProcessError::EmptyCommand
    );
    let missing = PathBuf::from("/definitely/missing/serenade-process-cwd");
    assert_eq!(
        Process::new("true").cwd(&missing).run().expect_err("cwd"),
        ProcessError::InvalidCwd { path: missing }
    );
}

#[test]
fn spawn_missing_binary_is_io() {
    let err = Process::new("serenade-process-no-such-bin-xyz")
        .run()
        .expect_err("io");
    assert!(matches!(err, ProcessError::Io { .. }));
    let again = Process::new("serenade-process-no-such-bin-xyz")
        .run()
        .expect_err("io2");
    assert_eq!(err, again);
}

#[test]
fn process_error_eq_covers_variants() {
    assert_eq!(ProcessError::EmptyCommand, ProcessError::EmptyCommand);
    assert_ne!(
        ProcessError::EmptyCommand,
        ProcessError::Failed {
            command: "x".into(),
            code: Some(1),
        }
    );
    let a = ProcessError::TimedOut {
        command: "sleep".into(),
        timeout: Duration::from_secs(1),
    };
    assert_eq!(a, a);
    assert_ne!(
        a,
        ProcessError::TimedOut {
            command: "sleep".into(),
            timeout: Duration::from_secs(2),
        }
    );
    let io = ProcessError::Io {
        command: "x".into(),
        source: std::io::Error::from(std::io::ErrorKind::NotFound),
    };
    assert_eq!(
        io,
        ProcessError::Io {
            command: "x".into(),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        }
    );
    assert_ne!(
        io,
        ProcessError::Io {
            command: "x".into(),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        }
    );
    assert_eq!(
        ProcessError::InvalidCwd {
            path: PathBuf::from("/a"),
        },
        ProcessError::InvalidCwd {
            path: PathBuf::from("/a"),
        }
    );
}

#[test]
fn args_builder_and_version() {
    let out = Process::new("printf")
        .args(["%s%s", "a", "b"])
        .must_run()
        .expect("printf");
    assert_eq!(out.stdout_string(), "ab");
    assert_ne!(crate::version(), "");
}

fn tempfile_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("serenade-process-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}
