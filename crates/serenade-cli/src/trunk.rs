//! Installs and runs Serenade's managed Trunk toolchain.
//!
//! Trunk is downloaded as a prebuilt binary from the rolling `trunk-itc-dev`
//! pre-release of the Interchouette Trunk fork. Before installation it must match
//! its published SHA-256 checksum, carry a GitHub build-provenance attestation
//! signed by the fork's release workflow (see `trunk/provenance.rs`), and report
//! `trunk <version>+itc.<revision>` for the attested commit. It is cached below
//! Serenade's tools directory together with its version and provenance.

mod provenance;

use std::{
    ffi::OsString,
    fs::{File, TryLockError},
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime},
};

use anyhow::{Context, bail};
use sha2::{Digest, Sha256};

const TRUNK_CHANNEL: &str = "trunk-itc-dev";
const TRUNK_RELEASE_URL: &str =
    "https://github.com/Interchouette-ITC/trunk/releases/download/trunk-itc-dev";
const RELEASE_URL_ENV: &str = "SERENADE_TRUNK_RELEASE_URL";
const REFRESH_ENV: &str = "SERENADE_TRUNK_REFRESH";
const CHECKSUM_MARKER: &str = ".serenade-trunk-sha256";
const VERSION_MARKER: &str = ".serenade-trunk-version";
const PROVENANCE_MARKER: &str = ".serenade-trunk-provenance";
const BINARY_MARKER: &str = ".serenade-trunk-bin-sha256";
const REFRESH_INTERVAL: Duration = Duration::from_hours(24);
const CHECKSUM_TIMEOUT: Duration = Duration::from_secs(15);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
const INSTALL_LOCK_TIMEOUT: Duration = Duration::from_secs(300);
const INSTALL_LOCK_RETRY: Duration = Duration::from_millis(250);

/// Returns a command for Serenade's managed Trunk binary, installing it on first use.
///
/// The binary comes from the fork's rolling `trunk-itc-dev` release and is stored
/// below `$SERENADE_HOME/tools` when configured, or `~/.serenade/tools` otherwise.
/// It is isolated from any globally installed `trunk` command. At most once a day
/// (or always with `SERENADE_TRUNK_REFRESH=1`) the published checksum is compared
/// with the installed build and a newer build is downloaded; when the release
/// cannot be reached, the installed build keeps being used. A newer build is only
/// installed once its GitHub build-provenance attestation verifies; if the
/// attestation cannot be fetched or verified, the command fails.
/// `SERENADE_TRUNK_RELEASE_URL` overrides the release download base URL (the
/// attestation is still checked against the fork's release workflow), and
/// `GITHUB_TOKEN`/`GH_TOKEN` authenticate the attestation lookup when set.
///
/// # Errors
///
/// Returns an error when the home directory cannot be resolved, the platform has no
/// published Trunk build, or no verified Trunk binary can be installed.
pub fn command() -> anyhow::Result<Command> {
    let tools_dir = serenade_home()?.join("tools");
    let install_dir = tools_dir.join(TRUNK_CHANNEL);
    let executable = trunk_executable(&install_dir);
    let marker = install_dir.join(CHECKSUM_MARKER);
    let forced = std::env::var_os(REFRESH_ENV).is_some_and(|value| value == "1");

    if forced || !is_current(&executable, &marker) {
        std::fs::create_dir_all(&tools_dir).with_context(|| {
            format!(
                "failed to create Serenade tools directory {}",
                tools_dir.display()
            )
        })?;
        if let Some(_lock) = acquire_install_lock(&tools_dir, &executable, &marker)?
            && (forced || !is_current(&executable, &marker))
        {
            update(&Release::for_host()?, &tools_dir, &install_dir)?;
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

/// Installed with a recorded checksum and verified provenance, and the executable
/// still matches the SHA-256 recorded when it was installed. Installs made before
/// provenance was checked, or whose executable was corrupted or replaced, are
/// reinstalled.
fn is_installed(executable: &Path, marker: &Path) -> bool {
    let Some(install_dir) = marker.parent() else {
        return false;
    };
    read_marker(marker).is_some()
        && read_provenance_commit(install_dir).is_some()
        && read_marker(&install_dir.join(BINARY_MARKER)).is_some_and(|recorded| {
            std::fs::read(executable).is_ok_and(|binary| sha256_hex(&binary) == recorded)
        })
}

/// Installed and checked against the release within the refresh interval.
fn is_current(executable: &Path, marker: &Path) -> bool {
    is_installed(executable, marker)
        && std::fs::metadata(marker)
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|modified| {
                SystemTime::now()
                    .duration_since(modified)
                    .is_ok_and(|age| age < REFRESH_INTERVAL)
            })
}

fn read_marker(marker: &Path) -> Option<String> {
    std::fs::read_to_string(marker)
        .ok()
        .and_then(|contents| parse_checksum(&contents))
}

fn write_marker(marker: &Path, checksum: &str) -> anyhow::Result<()> {
    std::fs::write(marker, format!("{checksum}\n"))
        .with_context(|| format!("failed to record Trunk checksum at {}", marker.display()))
}

/// Records the attested source commit and signer of an installed build.
fn write_provenance(install_dir: &Path, commit: &str) -> anyhow::Result<()> {
    let path = install_dir.join(PROVENANCE_MARKER);
    std::fs::write(
        &path,
        format!("commit {commit}\nsigner {}\n", provenance::SIGNER_WORKFLOW),
    )
    .with_context(|| format!("failed to record Trunk provenance at {}", path.display()))
}

fn read_provenance_commit(install_dir: &Path) -> Option<String> {
    std::fs::read_to_string(install_dir.join(PROVENANCE_MARKER))
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("commit "))
        .map(str::trim)
        .filter(|commit| commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .map(str::to_owned)
}

/// Parses a `sha256sum`-style file: the first token must be a SHA-256 hex digest.
fn parse_checksum(contents: &str) -> Option<String> {
    let digest = contents.split_whitespace().next()?.to_ascii_lowercase();
    (digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(digest)
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

/// Rust target triples of the published `trunk-itc-dev` binaries.
fn release_target(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Some("x86_64-unknown-linux-musl"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}

/// Release archive format: `.zip` for Windows targets, `.tar.gz` otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArchiveFormat {
    TarGz,
    Zip,
}

impl ArchiveFormat {
    fn for_target(target: &str) -> Self {
        if target.contains("-windows-") {
            Self::Zip
        } else {
            Self::TarGz
        }
    }

    const fn extension(self) -> &'static str {
        match self {
            Self::TarGz => "tar.gz",
            Self::Zip => "zip",
        }
    }
}

/// Verifies a downloaded archive's build provenance, given its SHA-256 digest.
type ProvenanceCheck = Box<dyn Fn(&str) -> anyhow::Result<provenance::Provenance>>;

struct Release {
    base_url: String,
    asset: String,
    format: ArchiveFormat,
    verify_provenance: ProvenanceCheck,
}

impl Release {
    fn for_host() -> anyhow::Result<Self> {
        let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
        let target = release_target(os, arch).ok_or_else(|| {
            anyhow::anyhow!(
                "no prebuilt Serenade Trunk ({TRUNK_CHANNEL}) is published for {os}/{arch}"
            )
        })?;
        let base_url =
            std::env::var(RELEASE_URL_ENV).unwrap_or_else(|_| TRUNK_RELEASE_URL.to_owned());
        Ok(Self::new(&base_url, target))
    }

    fn new(base_url: &str, target: &str) -> Self {
        let format = ArchiveFormat::for_target(target);
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            asset: format!("trunk-{target}.{}", format.extension()),
            format,
            verify_provenance: Box::new(|sha256| {
                provenance::verify_archive(
                    provenance::ATTESTATIONS_URL,
                    sha256,
                    provenance::SIGNER_WORKFLOW,
                )
            }),
        }
    }

    fn archive_url(&self) -> String {
        format!("{}/{}", self.base_url, self.asset)
    }

    fn checksum_url(&self) -> String {
        format!("{}.sha256", self.archive_url())
    }

    fn fetch_checksum(&self) -> anyhow::Result<String> {
        let url = self.checksum_url();
        let body = http_get(&url, CHECKSUM_TIMEOUT)?;
        std::str::from_utf8(&body)
            .ok()
            .and_then(parse_checksum)
            .ok_or_else(|| anyhow::anyhow!("malformed Trunk checksum file at {url}"))
    }
}

/// Sends a GET with a blocking client on its own thread, so callers may run
/// inside an async runtime. Returns the HTTP status and body.
fn http_fetch(
    url: &str,
    timeout: Duration,
    headers: Vec<(&'static str, String)>,
) -> anyhow::Result<(u16, Vec<u8>)> {
    let owned = url.to_owned();
    thread::spawn(move || -> anyhow::Result<(u16, Vec<u8>)> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .user_agent(concat!("serenade-cli/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let mut request = client.get(&owned);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request.send()?;
        let status = response.status().as_u16();
        Ok((status, response.bytes()?.to_vec()))
    })
    .join()
    .map_err(|_| anyhow::anyhow!("download thread panicked"))?
    .with_context(|| format!("failed to download {url}"))
}

fn http_get(url: &str, timeout: Duration) -> anyhow::Result<Vec<u8>> {
    let (status, body) = http_fetch(url, timeout, Vec::new())?;
    if !(200..300).contains(&status) {
        bail!("failed to download {url}: HTTP {status}");
    }
    Ok(body)
}

/// Installs the published build unless the installed one already matches it.
fn update(release: &Release, tools_dir: &Path, install_dir: &Path) -> anyhow::Result<()> {
    let executable = trunk_executable(install_dir);
    let marker = install_dir.join(CHECKSUM_MARKER);
    let installed = is_installed(&executable, &marker)
        .then(|| read_marker(&marker))
        .flatten();

    let expected = match release.fetch_checksum() {
        Ok(checksum) => checksum,
        Err(error) => {
            let Some(installed) = installed else {
                return Err(error.context(format!(
                    "failed to install Serenade's Trunk ({TRUNK_CHANNEL})"
                )));
            };
            eprintln!(
                "warning: could not check for a newer Serenade Trunk build ({error:#}); using the installed one"
            );
            // Postpone the next check instead of retrying on every offline build.
            return write_marker(&marker, &installed);
        }
    };
    if installed.as_deref() == Some(expected.as_str()) {
        return write_marker(&marker, &expected);
    }

    eprintln!(
        "Downloading Serenade's Trunk ({TRUNK_CHANNEL}) from {}",
        release.archive_url()
    );
    let archive = http_get(&release.archive_url(), DOWNLOAD_TIMEOUT)?;
    let actual = sha256_hex(&archive);
    if actual != expected {
        bail!(
            "checksum mismatch for {} (expected {expected}, got {actual}); the release may be updating, retry shortly",
            release.archive_url()
        );
    }
    let provenance = (release.verify_provenance)(&actual).with_context(|| {
        format!(
            "refusing to install {}: its build provenance could not be verified",
            release.archive_url()
        )
    })?;

    let staging = tools_dir.join(format!(".{TRUNK_CHANNEL}.staging"));
    remove_dir_if_exists(&staging)?;
    let staged_executable = trunk_executable(&staging);
    unpack_executable(release.format, &archive, &staged_executable)?;
    let version = verify_version(&staged_executable)?;
    if !fork_revision(&version).is_some_and(|revision| provenance.commit.starts_with(revision)) {
        bail!(
            "downloaded Trunk reports {version:?}, but its attested build is commit {}",
            provenance.commit
        );
    }
    let version_marker = staging.join(VERSION_MARKER);
    std::fs::write(&version_marker, format!("{version}\n")).with_context(|| {
        format!(
            "failed to record Trunk version at {}",
            version_marker.display()
        )
    })?;
    write_provenance(&staging, &provenance.commit)?;
    let binary = std::fs::read(&staged_executable)
        .with_context(|| format!("failed to read {}", staged_executable.display()))?;
    write_marker(&staging.join(BINARY_MARKER), &sha256_hex(&binary))?;
    write_marker(&staging.join(CHECKSUM_MARKER), &expected)?;

    replace_install(&staging, install_dir)?;
    eprintln!(
        "Installed {version} ({TRUNK_CHANNEL}, sha256 {expected}, provenance verified: commit {})",
        provenance.commit
    );
    Ok(())
}

/// Moves a staged install into place file by file. Each rename atomically replaces
/// its target, so a concurrent caller never finds the executable missing, and runs
/// already started keep their binary. The binary checksum follows the executable
/// and the release checksum goes last, so an interrupted replacement fails the
/// `is_installed` check and is reinstalled.
fn replace_install(staging: &Path, install_dir: &Path) -> anyhow::Result<()> {
    let executable = trunk_executable(install_dir);
    let bin_dir = executable
        .parent()
        .context("Trunk executable has no parent")?;
    std::fs::create_dir_all(bin_dir)
        .with_context(|| format!("failed to create {}", bin_dir.display()))?;
    let files = [
        (trunk_executable(staging), executable),
        (staging.join(BINARY_MARKER), install_dir.join(BINARY_MARKER)),
        (
            staging.join(VERSION_MARKER),
            install_dir.join(VERSION_MARKER),
        ),
        (
            staging.join(PROVENANCE_MARKER),
            install_dir.join(PROVENANCE_MARKER),
        ),
        (
            staging.join(CHECKSUM_MARKER),
            install_dir.join(CHECKSUM_MARKER),
        ),
    ];
    for (from, to) in files {
        std::fs::rename(&from, &to).with_context(|| {
            format!(
                "failed to move the downloaded Trunk into {} (is Trunk running?)",
                to.display()
            )
        })?;
    }
    remove_dir_if_exists(staging)
}

fn remove_dir_if_exists(directory: &Path) -> anyhow::Result<()> {
    match std::fs::remove_dir_all(directory) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(error).with_context(|| format!("failed to remove {}", directory.display()))
        }
        _ => Ok(()),
    }
}

/// Extracts the Trunk executable from a release archive into `executable`.
fn unpack_executable(
    format: ArchiveFormat,
    archive: &[u8],
    executable: &Path,
) -> anyhow::Result<()> {
    let name = executable
        .file_name()
        .context("Trunk executable path has no file name")?;
    let contents = match format {
        ArchiveFormat::TarGz => read_tar_gz_entry(archive, name)?,
        ArchiveFormat::Zip => read_zip_entry(archive, name)?,
    };
    let Some(contents) = contents else {
        bail!(
            "the Trunk archive does not contain {}",
            name.to_string_lossy()
        );
    };

    let parent = executable
        .parent()
        .context("Trunk executable has no parent")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;
    std::fs::write(executable, contents)
        .with_context(|| format!("failed to write {}", executable.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(executable, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn read_tar_gz_entry(archive: &[u8], name: &std::ffi::OsStr) -> anyhow::Result<Option<Vec<u8>>> {
    let mut entries = tar::Archive::new(flate2::read::GzDecoder::new(archive));
    for entry in entries
        .entries()
        .context("failed to read the Trunk archive")?
    {
        let mut entry = entry.context("failed to read a Trunk archive entry")?;
        if entry.path()?.file_name() != Some(name) || !entry.header().entry_type().is_file() {
            continue;
        }
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents)?;
        return Ok(Some(contents));
    }
    Ok(None)
}

fn read_zip_entry(archive: &[u8], name: &std::ffi::OsStr) -> anyhow::Result<Option<Vec<u8>>> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive))
        .context("failed to read the Trunk archive")?;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .context("failed to read a Trunk archive entry")?;
        let matches = entry
            .enclosed_name()
            .is_some_and(|path| path.file_name() == Some(name));
        if !matches || !entry.is_file() {
            continue;
        }
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents)?;
        return Ok(Some(contents));
    }
    Ok(None)
}

/// Runs `trunk --version` and requires the fork's `trunk <version>+itc.<revision>`
/// output, so every installed build identifies the commit it was built from.
fn verify_version(executable: &Path) -> anyhow::Result<String> {
    let output = Command::new(executable)
        .arg("--version")
        .output()
        .with_context(|| format!("failed to run {}", executable.display()))?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() || fork_revision(&version).is_none() {
        bail!(
            "downloaded Trunk failed its version check (status {}, output {version:?}); expected `trunk <version>+itc.<revision>`",
            output.status
        );
    }
    Ok(version)
}

/// Extracts the build revision from `trunk <version>+itc.<revision>`.
fn fork_revision(version: &str) -> Option<&str> {
    let (release, revision) = version.strip_prefix("trunk ")?.split_once("+itc.")?;
    (!release.is_empty()
        && !revision.is_empty()
        && revision
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'))
    .then_some(revision)
}

fn acquire_install_lock(
    tools_dir: &Path,
    executable: &Path,
    marker: &Path,
) -> anyhow::Result<Option<InstallLock>> {
    let lock_path = tools_dir.join(format!("{TRUNK_CHANNEL}.install-lock"));
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
        let lock_path = temp.path().join(format!("{TRUNK_CHANNEL}.install-lock"));
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
    fn install_requires_checksum_and_provenance_records() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let executable = temp.path().join("trunk");
        let marker = temp.path().join(CHECKSUM_MARKER);
        std::fs::write(&executable, "trunk").expect("executable placeholder");
        write_marker(&marker, &"a".repeat(64)).expect("checksum marker");
        assert!(
            !is_installed(&executable, &marker),
            "an install without verified provenance is replaced"
        );
        write_provenance(temp.path(), &"c".repeat(40)).expect("provenance marker");
        assert_eq!(read_provenance_commit(temp.path()), Some("c".repeat(40)));
        assert!(
            !is_installed(&executable, &marker),
            "an install without a binary checksum is replaced"
        );
        write_marker(&temp.path().join(BINARY_MARKER), &sha256_hex(b"trunk"))
            .expect("binary marker");
        assert!(is_installed(&executable, &marker));
        assert!(is_current(&executable, &marker));
        std::fs::write(&executable, "trunc").expect("corrupt executable");
        assert!(
            !is_installed(&executable, &marker),
            "a modified executable is replaced"
        );
        std::fs::write(&executable, "trunk").expect("restore executable");
        std::fs::write(&marker, "v0.22.1-itc.5\n").expect("legacy tag marker");
        assert!(!is_installed(&executable, &marker));
    }

    #[test]
    fn version_must_carry_the_fork_build_revision() {
        assert_eq!(
            fork_revision("trunk 0.22.1+itc.0123456789ab"),
            Some("0123456789ab")
        );
        assert_eq!(fork_revision("trunk 0.22.1"), None);
        assert_eq!(fork_revision("trunk 0.22.1+itc"), None);
        assert_eq!(fork_revision("trunk 0.22.1+itc."), None);
        assert_eq!(fork_revision("trunk +itc.abc"), None);
        assert_eq!(fork_revision("trunk 0.22.1+itc.ab/c"), None);
        assert_eq!(fork_revision("other 0.22.1+itc.abc"), None);
    }

    #[test]
    fn checksum_files_accept_bare_and_sha256sum_formats() {
        let digest = "AB".repeat(32);
        assert_eq!(
            parse_checksum(&format!("{digest}\n")),
            Some("ab".repeat(32))
        );
        assert_eq!(
            parse_checksum(&format!("{digest}  trunk-x.tar.gz\n")),
            Some("ab".repeat(32))
        );
        assert_eq!(parse_checksum("not-a-digest"), None);
        assert_eq!(parse_checksum(""), None);
    }

    #[test]
    fn published_targets_cover_supported_hosts() {
        assert_eq!(
            release_target("linux", "x86_64"),
            Some("x86_64-unknown-linux-musl")
        );
        assert_eq!(
            release_target("macos", "x86_64"),
            Some("x86_64-apple-darwin")
        );
        assert_eq!(
            release_target("macos", "aarch64"),
            Some("aarch64-apple-darwin")
        );
        assert_eq!(
            release_target("windows", "x86_64"),
            Some("x86_64-pc-windows-msvc")
        );
        assert_eq!(release_target("linux", "aarch64"), None);
        let release = Release::new("https://example.test/download/", "x86_64-apple-darwin");
        assert_eq!(
            release.checksum_url(),
            "https://example.test/download/trunk-x86_64-apple-darwin.tar.gz.sha256"
        );
        assert_eq!(release.format, ArchiveFormat::TarGz);
        let windows = Release::new("https://example.test/download", "x86_64-pc-windows-msvc");
        assert_eq!(
            windows.archive_url(),
            "https://example.test/download/trunk-x86_64-pc-windows-msvc.zip"
        );
        assert_eq!(windows.format, ArchiveFormat::Zip);
    }

    #[test]
    fn unpacks_the_executable_from_a_zip_archive() {
        use std::io::Write;

        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer.add_directory("docs/", options).expect("directory");
        writer
            .start_file("docs/README.txt", options)
            .expect("readme");
        writer.write_all(b"not trunk").expect("write readme");
        writer.start_file("trunk.exe", options).expect("trunk");
        writer.write_all(b"MZ trunk binary").expect("write trunk");
        let archive = writer.finish().expect("finish zip").into_inner();

        let temp = tempfile::tempdir().expect("temporary directory");
        let executable = temp.path().join("bin/trunk.exe");
        unpack_executable(ArchiveFormat::Zip, &archive, &executable).expect("unpack zip");
        assert_eq!(
            std::fs::read(&executable).expect("unpacked"),
            b"MZ trunk binary"
        );
        let missing = unpack_executable(ArchiveFormat::Zip, &archive, &temp.path().join("trunk"))
            .expect_err("missing entry");
        assert!(format!("{missing:#}").contains("does not contain trunk"));
        assert!(
            unpack_executable(ArchiveFormat::TarGz, &archive, &executable).is_err(),
            "a zip is not a tar.gz"
        );
    }

    #[cfg(unix)]
    mod download {
        use std::{
            collections::HashMap,
            io::{BufRead, BufReader, Write},
            net::TcpListener,
            sync::{Arc, Mutex},
        };

        use super::*;

        type Files = Arc<Mutex<HashMap<String, Vec<u8>>>>;

        fn serve(files: Files) -> String {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind release server");
            let address = listener.local_addr().expect("server address");
            thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { continue };
                    let mut request = String::new();
                    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
                    reader.read_line(&mut request).expect("request line");
                    let mut header = String::new();
                    while reader.read_line(&mut header).is_ok_and(|read| read > 2) {
                        header.clear();
                    }
                    let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
                    let body = files.lock().expect("files").get(&path).cloned();
                    let (status, body) =
                        body.map_or_else(|| ("404 Not Found", Vec::new()), |body| ("200 OK", body));
                    let _ = write!(
                        stream,
                        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(&body);
                }
            });
            format!("http://{address}/download")
        }

        fn archive(version: &str) -> Vec<u8> {
            let script = format!("#!/bin/sh\necho 'trunk {version}'\n");
            let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
                Vec::new(),
                flate2::Compression::fast(),
            ));
            let mut header = tar::Header::new_gnu();
            header.set_size(script.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, "trunk", script.as_bytes())
                .expect("append trunk");
            builder
                .into_inner()
                .expect("finish tar")
                .finish()
                .expect("finish gzip")
        }

        fn publish(files: &Files, archive: Vec<u8>, checksum: &str) {
            files.lock().expect("files").extend([
                ("/download/trunk-test.tar.gz".to_owned(), archive),
                (
                    "/download/trunk-test.tar.gz.sha256".to_owned(),
                    format!("{checksum}\n").into_bytes(),
                ),
            ]);
        }

        /// Archive digest -> attested commit, standing in for GitHub attestations.
        type Attested = Arc<Mutex<HashMap<String, String>>>;

        fn attest(attested: &Attested, archive: &[u8], commit: &str) {
            attested
                .lock()
                .expect("attested")
                .insert(sha256_hex(archive), commit.to_owned());
        }

        fn attested_release(base_url: &str, attested: &Attested) -> Release {
            let mut release = Release::new(base_url, "test");
            let attested = Arc::clone(attested);
            release.verify_provenance = Box::new(move |sha256| {
                attested
                    .lock()
                    .expect("attested")
                    .get(sha256)
                    .map(|commit| provenance::Provenance {
                        commit: commit.clone(),
                    })
                    .ok_or_else(|| anyhow::anyhow!("no build provenance attestation"))
            });
            release
        }

        /// Version reported by the installed binary, which must match the recorded one.
        fn installed_version(install_dir: &Path) -> String {
            let version =
                verify_version(&trunk_executable(install_dir)).expect("installed version");
            let recorded =
                std::fs::read_to_string(install_dir.join(VERSION_MARKER)).expect("version marker");
            assert_eq!(recorded.trim(), version);
            assert!(read_provenance_commit(install_dir).is_some());
            version
        }

        #[test]
        fn downloads_verifies_refreshes_and_survives_an_offline_release() {
            let temp = tempfile::tempdir().expect("temporary directory");
            let install_dir = temp.path().join(TRUNK_CHANNEL);
            let marker = install_dir.join(CHECKSUM_MARKER);
            let files = Files::default();
            let attested = Attested::default();
            let release = attested_release(&serve(Arc::clone(&files)), &attested);

            let first = archive("0.22.1+itc.aaaaaaaaaaaa");
            attest(&attested, &first, &"a".repeat(40));
            publish(&files, first.clone(), &sha256_hex(&first));
            update(&release, temp.path(), &install_dir).expect("first install");
            assert_eq!(
                installed_version(&install_dir),
                "trunk 0.22.1+itc.aaaaaaaaaaaa"
            );
            assert_eq!(read_marker(&marker), Some(sha256_hex(&first)));
            assert_eq!(read_provenance_commit(&install_dir), Some("a".repeat(40)));
            assert!(!temp.path().join(".trunk-itc-dev.staging").exists());

            let second = archive("0.22.1+itc.bbbbbbbbbbbb");
            publish(&files, second.clone(), &sha256_hex(&first));
            update(&release, temp.path(), &install_dir)
                .expect("unchanged checksum keeps the install");
            assert_eq!(
                installed_version(&install_dir),
                "trunk 0.22.1+itc.aaaaaaaaaaaa"
            );

            publish(&files, second.clone(), &"0".repeat(64));
            let error = update(&release, temp.path(), &install_dir)
                .expect_err("tampered archive is rejected");
            assert!(format!("{error:#}").contains("checksum mismatch"));
            assert_eq!(
                installed_version(&install_dir),
                "trunk 0.22.1+itc.aaaaaaaaaaaa"
            );

            publish(&files, second.clone(), &sha256_hex(&second));
            let error = update(&release, temp.path(), &install_dir)
                .expect_err("an unattested build is refused");
            assert!(format!("{error:#}").contains("build provenance could not be verified"));
            assert_eq!(
                installed_version(&install_dir),
                "trunk 0.22.1+itc.aaaaaaaaaaaa"
            );

            attest(&attested, &second, &"c".repeat(40));
            let error = update(&release, temp.path(), &install_dir)
                .expect_err("revision and attested commit must agree");
            assert!(format!("{error:#}").contains("attested build is commit"));
            assert_eq!(
                installed_version(&install_dir),
                "trunk 0.22.1+itc.aaaaaaaaaaaa"
            );

            attest(&attested, &second, &"b".repeat(40));
            update(&release, temp.path(), &install_dir).expect("newer build installs");
            assert_eq!(
                installed_version(&install_dir),
                "trunk 0.22.1+itc.bbbbbbbbbbbb"
            );

            files.lock().expect("files").clear();
            update(&release, temp.path(), &install_dir).expect("offline keeps the install");
            assert_eq!(
                installed_version(&install_dir),
                "trunk 0.22.1+itc.bbbbbbbbbbbb"
            );

            let executable = trunk_executable(&install_dir);
            let mut corrupted = std::fs::read(&executable).expect("installed executable");
            corrupted.push(b'\n');
            std::fs::write(&executable, corrupted).expect("corrupt executable");
            assert!(!is_installed(&executable, &marker));
            let error = update(&release, temp.path(), &install_dir)
                .expect_err("offline with a corrupted install fails");
            assert!(format!("{error:#}").contains("failed to install"));

            publish(&files, second.clone(), &sha256_hex(&second));
            update(&release, temp.path(), &install_dir).expect("corrupted install is replaced");
            assert!(is_installed(&executable, &marker));
            assert_eq!(
                installed_version(&install_dir),
                "trunk 0.22.1+itc.bbbbbbbbbbbb"
            );
            files.lock().expect("files").clear();
            let empty = temp.path().join("empty");
            assert!(update(&release, temp.path(), &empty).is_err());

            let upstream = archive("0.22.1");
            attest(&attested, &upstream, &"d".repeat(40));
            publish(&files, upstream.clone(), &sha256_hex(&upstream));
            let error = update(&release, temp.path(), &install_dir)
                .expect_err("a build without fork revision is rejected");
            assert!(format!("{error:#}").contains("+itc.<revision>"));
            assert_eq!(
                installed_version(&install_dir),
                "trunk 0.22.1+itc.bbbbbbbbbbbb"
            );
        }

        #[test]
        fn rejects_binaries_that_are_not_trunk() {
            let temp = tempfile::tempdir().expect("temporary directory");
            let executable = temp.path().join("bin/trunk");
            let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
                Vec::new(),
                flate2::Compression::fast(),
            ));
            let script = b"#!/bin/sh\necho 'other 1.0'\n";
            let mut header = tar::Header::new_gnu();
            header.set_size(script.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder
                .append_data(&mut header, "trunk", &script[..])
                .expect("append");
            let archive = builder.into_inner().expect("tar").finish().expect("gzip");
            unpack_executable(ArchiveFormat::TarGz, &archive, &executable).expect("unpack");
            assert!(verify_version(&executable).is_err());
            assert!(
                unpack_executable(
                    ArchiveFormat::TarGz,
                    &archive,
                    &temp.path().join("bin/trunk.exe")
                )
                .is_err()
            );
        }
    }
}
