//! Installs and runs Serenade's pinned Trunk toolchain.

use std::{
    ffi::OsString,
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
    let started = Instant::now();

    loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(_) => return Ok(Some(InstallLock(lock_path))),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if is_installed(executable, marker) {
                    return Ok(None);
                }
                if started.elapsed() >= INSTALL_LOCK_TIMEOUT {
                    bail!(
                        "timed out waiting for Serenade's Trunk installation lock at {}",
                        lock_path.display()
                    );
                }
                thread::sleep(INSTALL_LOCK_RETRY);
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to create Serenade's Trunk installation lock {}",
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

struct InstallLock(PathBuf);

impl Drop for InstallLock {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.0) {
            eprintln!(
                "Warning: failed to remove Trunk installation lock {}: {error}",
                self.0.display()
            );
        }
    }
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
