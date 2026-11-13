//! Installs and runs Serenade's pinned Trunk toolchain.

use std::{
    ffi::OsString,
    fs::{File, TryLockError},
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, bail};

const TRUNK_REPOSITORY: &str = "https://github.com/Interchouette-ITC/trunk.git";
const TRUNK_TAG: &str = "v0.22.1-itc.2";
const INSTALL_LOCK_TIMEOUT: Duration = Duration::from_secs(300);
const INSTALL_LOCK_RETRY: Duration = Duration::from_millis(250);

/// Returns a command for Serenade's pinned Trunk binary, installing it on first use.
///
/// The installation is stored below `$SERENADE_HOME/tools` when configured, or
/// `~/.serenade/tools` otherwise. It is isolated from any globally installed
/// `trunk` command.
///
/// # Errors
///
/// Returns an error when the home directory cannot be resolved, Cargo cannot be
/// started, the installation fails, or the pinned executable is unavailable.
pub fn command() -> anyhow::Result<Command> {
    let tools_dir = serenade_home()?.join("tools");
    let install_dir = tools_dir.join(format!("trunk-{TRUNK_TAG}"));
    let executable = trunk_executable(&install_dir);
    let marker = install_dir.join(".serenade-trunk-tag");

    if !is_installed(&executable, &marker) {
        std::fs::create_dir_all(&tools_dir).with_context(|| {
            format!(
                "failed to create Serenade tools directory {}",
                tools_dir.display()
            )
        })?;
        if let Some(_lock) = acquire_install_lock(&tools_dir, &executable, &marker)?
            && !is_installed(&executable, &marker)
        {
            install(&install_dir, &executable, &marker)?;
        }
    }

    Ok(Command::new(executable))
}

/// Builds the app in `directory` with Serenade's Trunk and validates the fresh
/// asset manifest and all referenced output files before returning.
///
/// `args` are passed after Trunk's `build` subcommand, e.g.
/// `["--features", "hydrate"]`.
///
/// # Errors
///
/// Returns an error if Trunk fails or produces missing, stale, malformed, or
/// incompatible output artifacts.
pub fn build(directory: &Path, args: &[&str]) -> anyhow::Result<()> {
    let dist = crate::html_output::dist_directory_for_build(directory, args)?;
    crate::html_output::clear_trunk_asset_manifest(&dist)?;

    let status = command()?
        .arg("build")
        .args(args)
        .current_dir(directory)
        .status()
        .with_context(|| {
            format!(
                "failed to start Serenade's Trunk build in {}",
                directory.display()
            )
        })?;
    if !status.success() {
        bail!(
            "Serenade's Trunk build failed in {} with status {status}",
            directory.display()
        );
    }

    crate::html_output::validate_trunk_asset_manifest(&dist)?;
    Ok(())
}

fn serenade_home() -> anyhow::Result<PathBuf> {
    if let Some(home) = std::env::var_os("SERENADE_HOME") {
        return Ok(PathBuf::from(home));
    }

    #[cfg(windows)]
    let home = std::env::var_os("USERPROFILE");
    #[cfg(not(windows))]
    let home = std::env::var_os("HOME");

    home.map(PathBuf::from)
        .map(|home| home.join(".serenade"))
        .ok_or_else(|| anyhow::anyhow!("could not resolve Serenade home; set SERENADE_HOME"))
}

fn trunk_executable(install_dir: &Path) -> PathBuf {
    let name = if cfg!(windows) { "trunk.exe" } else { "trunk" };
    install_dir.join("bin").join(name)
}

fn is_installed(executable: &Path, marker: &Path) -> bool {
    executable.is_file() && std::fs::read_to_string(marker).is_ok_and(|tag| tag.trim() == TRUNK_TAG)
}

fn acquire_install_lock(
    tools_dir: &Path,
    executable: &Path,
    marker: &Path,
) -> anyhow::Result<Option<InstallLock>> {
    let lock_path = tools_dir.join(format!("trunk-{TRUNK_TAG}.install-lock"));
    // OS advisory lock: released by the kernel when the holder exits or is killed, so an
    // abandoned lock file is simply re-locked. Keep the inode in place: deleting it could
    // let two installers lock different files.
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .with_context(|| {
            format!(
                "failed to open Trunk installation lock {}",
                lock_path.display()
            )
        })?;
    let started = Instant::now();

    loop {
        match file.try_lock() {
            Ok(()) => return Ok(Some(InstallLock { _file: file })),
            Err(TryLockError::WouldBlock) => {
                if is_installed(executable, marker) {
                    return Ok(None);
                }
                if started.elapsed() >= INSTALL_LOCK_TIMEOUT {
                    bail!(
                        "timed out waiting for a running process holding Serenade's Trunk installation lock at {}",
                        lock_path.display()
                    );
                }
                thread::sleep(INSTALL_LOCK_RETRY);
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to acquire Serenade's Trunk installation lock {}",
                        lock_path.display()
                    )
                });
            }
        }
    }
}

fn install(install_dir: &Path, executable: &Path, marker: &Path) -> anyhow::Result<()> {
    eprintln!(
        "Installing Serenade's pinned Trunk release ({TRUNK_TAG}); this is only needed once."
    );
    let status = Command::new("cargo")
        .args(["install", "--root"])
        .arg(install_dir)
        .args([
            "--git",
            TRUNK_REPOSITORY,
            "--tag",
            TRUNK_TAG,
            "--locked",
            "--force",
            "trunk",
        ])
        .status()
        .context("failed to start Cargo while installing Serenade's Trunk toolchain")?;
    if !status.success() {
        bail!("Cargo failed to install Serenade's pinned Trunk toolchain with status {status}");
    }
    if !executable.is_file() {
        bail!(
            "Cargo reported a successful Trunk install but the executable is missing: {}",
            executable.display()
        );
    }
    std::fs::write(marker, format!("{TRUNK_TAG}\n"))
        .with_context(|| format!("failed to record Trunk release tag at {}", marker.display()))?;
    Ok(())
}

struct InstallLock {
    _file: File,
}

/// Converts program arguments into an owned vector, useful to forward CLI args.
#[must_use]
pub fn args_from_env() -> Vec<OsString> {
    std::env::args_os().skip(1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_lock_subprocess() {
        let Some(directory) = std::env::var_os("SERENADE_TEST_INSTALL_LOCK_DIR") else {
            return;
        };
        let directory = PathBuf::from(directory);
        let _lock =
            acquire_install_lock(&directory, &directory.join("trunk"), &directory.join("tag"))
                .expect("child acquires installation lock");
        std::fs::write(directory.join("ready"), "").expect("signal locked");
        thread::sleep(Duration::from_secs(10));
    }

    #[test]
    fn installation_lock_is_released_when_its_process_is_killed() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let mut child = Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", "trunk::tests::install_lock_subprocess"])
            .env("SERENADE_TEST_INSTALL_LOCK_DIR", temp.path())
            .spawn()
            .expect("spawn installer");
        let ready = (0..100).any(|_| {
            if temp.path().join("ready").is_file() {
                true
            } else {
                thread::sleep(Duration::from_millis(50));
                false
            }
        });
        child.kill().expect("kill installer");
        child.wait().expect("reap installer");
        assert!(ready, "child must acquire the lock before it is killed");
        let _lock = acquire_install_lock(
            temp.path(),
            &temp.path().join("trunk"),
            &temp.path().join("tag"),
        )
        .expect("recover killed installer lock")
        .expect("installation required");
    }

    #[test]
    fn abandoned_install_lock_file_is_reused_and_never_unlinked() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let lock_path = temp.path().join(format!("trunk-{TRUNK_TAG}.install-lock"));
        std::fs::write(&lock_path, "abandoned installer").expect("stale lock");
        let executable = temp.path().join("trunk");
        let marker = temp.path().join("tag");
        let lock = acquire_install_lock(temp.path(), &executable, &marker)
            .expect("recover stale lock")
            .expect("installation required");
        let contender = File::options()
            .write(true)
            .open(&lock_path)
            .expect("open contender");
        assert!(matches!(
            contender.try_lock(),
            Err(TryLockError::WouldBlock)
        ));
        drop(lock);
        assert!(lock_path.is_file());
        contender.try_lock().expect("released on owner drop");
    }

    #[test]
    fn managed_executable_is_isolated_under_its_revision_directory() {
        let path = trunk_executable(Path::new("/serenade/tools/trunk-revision"));
        if cfg!(windows) {
            assert_eq!(
                path,
                Path::new("/serenade/tools/trunk-revision/bin/trunk.exe")
            );
        } else {
            assert_eq!(path, Path::new("/serenade/tools/trunk-revision/bin/trunk"));
        }
    }

    #[test]
    fn install_marker_must_match_the_pinned_release_tag() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let executable = temp.path().join("trunk");
        let marker = temp.path().join("revision");
        std::fs::write(&executable, "").expect("executable placeholder");
        std::fs::write(&marker, format!("{TRUNK_TAG}\n")).expect("release tag marker");
        assert!(is_installed(&executable, &marker));
        std::fs::write(&marker, "another-revision").expect("replace revision marker");
        assert!(!is_installed(&executable, &marker));
    }
}
