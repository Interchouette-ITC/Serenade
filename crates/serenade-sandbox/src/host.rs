//! Wasmer WASIX runtime, package download, and module/package runners.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use bytes::Bytes;
use shared_buffer::OwnedBuffer;
use tokio::runtime::Handle;
use virtual_fs::{AsyncReadExt, AsyncSeekExt, StaticFile};
use wasmer::Module;
use wasmer_package::utils::from_bytes;
use wasmer_types::ModuleHash;
use wasmer_wasix::PluggableRuntime;
use wasmer_wasix::Runtime;
use wasmer_wasix::bin_factory::BinaryPackage;
use wasmer_wasix::runners::wasi::{RuntimeOrEngine, WasiRunner};
use wasmer_wasix::runtime::module_cache::{FileSystemCache, ModuleCache, SharedCache};
use wasmer_wasix::runtime::package_loader::BuiltinPackageLoader;
use wasmer_wasix::runtime::task_manager::tokio::TokioTaskManager;

use crate::output::GuestOutput;
use crate::package_run::PackageRun;

/// Resolves the Wasmer package/module cache root.
///
/// Uses `SERENADE_WASMER_CACHE` when set; otherwise `.wasmer` under the current
/// working directory (products may wrap this with their own default).
#[must_use]
pub fn wasmer_cache_root() -> PathBuf {
    wasmer_cache_root_from(std::env::var_os("SERENADE_WASMER_CACHE"))
}

/// Resolves cache root from an optional override path.
#[must_use]
pub fn wasmer_cache_root_from(override_path: Option<std::ffi::OsString>) -> PathBuf {
    if let Some(path) = override_path {
        return PathBuf::from(path);
    }
    PathBuf::from(".wasmer")
}

/// Downloads (or reads cached) a Wasmer webc package.
///
/// # Errors
///
/// Returns an error when the cache cannot be read/written or the download fails.
pub async fn load_webc_from(
    cache_root: &Path,
    package_url: &str,
    cache_name: &str,
) -> Result<Bytes> {
    let downloads = cache_root.join("downloads");
    let cache_path = downloads.join(cache_name);
    if cache_path.is_file() {
        return Ok(std::fs::read(&cache_path)
            .with_context(|| format!("read cached webc {}", cache_path.display()))?
            .into());
    }
    std::fs::create_dir_all(&downloads)
        .with_context(|| format!("create {}", downloads.display()))?;

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(300))
        .build()
        .context("build HTTP client for Wasmer package download")?;
    let response = client
        .get(package_url)
        .header("Accept", "application/webc")
        .send()
        .await
        .with_context(|| format!("GET {package_url}"))?;
    if !response.status().is_success() {
        bail!(
            "download {package_url} failed with HTTP {}",
            response.status()
        );
    }
    let body = response.bytes().await.context("read Wasmer webc body")?;
    ensure_webc_payload(&body)?;
    std::fs::write(&cache_path, &body)
        .with_context(|| format!("cache webc at {}", cache_path.display()))?;
    Ok(body)
}

/// Runs a Wasmer registry package command with stdin bytes; returns guest stdio.
///
/// # Errors
///
/// Returns an error when the package cannot be loaded or the runner task fails
/// to join (guest failure is reported via [`GuestOutput::success`]).
pub async fn run_package(request: PackageRun<'_>) -> Result<GuestOutput> {
    let PackageRun {
        stdin_bytes,
        cache_root,
        package_url,
        cache_name,
        command,
        args,
    } = request;
    let webc = load_webc_from(cache_root, package_url, cache_name).await?;
    let container =
        from_bytes(webc).with_context(|| format!("decode Wasmer webc {package_url}"))?;
    let (runtime, tasks) = build_runtime(cache_root)?;
    let pkg = BinaryPackage::from_webc(&container, &runtime)
        .await
        .context("load BinaryPackage from webc")?;

    let stdin = StaticFile::new(OwnedBuffer::from_bytes(stdin_bytes.to_vec()));
    let mut stdout = virtual_fs::ArcFile::new(Box::<virtual_fs::BufferFile>::default());
    let mut stderr = virtual_fs::ArcFile::new(Box::<virtual_fs::BufferFile>::default());
    let stdout_guest = stdout.clone();
    let stderr_guest = stderr.clone();
    let runtime = Arc::new(runtime);
    let command = command.to_owned();

    let join = tokio::task::spawn_blocking(move || {
        let _guard = tasks.runtime_handle().enter();
        WasiRunner::new()
            .with_args(args)
            .with_stdin(Box::new(stdin) as Box<_>)
            .with_stdout(Box::new(stdout_guest) as Box<_>)
            .with_stderr(Box::new(stderr_guest) as Box<_>)
            .run_command(&command, &pkg, RuntimeOrEngine::Runtime(runtime))
    });

    let run_result = join.await.context("join wasmer package task")?;
    read_guest_output(&mut stdout, &mut stderr, run_result.is_ok()).await
}

/// Compiles and runs a WASI module from bytes with stdin; returns guest stdio.
///
/// # Errors
///
/// Returns an error when the module cannot be compiled or the runner task fails
/// to join.
pub async fn run_module(
    program_name: &str,
    wasm_bytes: &[u8],
    stdin_bytes: &[u8],
    cache_root: &Path,
) -> Result<GuestOutput> {
    let (runtime, tasks) = build_runtime(cache_root)?;
    let engine = runtime.engine();
    let module = Module::new(&engine, wasm_bytes).context("compile WASI module")?;
    let module_hash = ModuleHash::new(wasm_bytes);

    let stdin = StaticFile::new(OwnedBuffer::from_bytes(stdin_bytes.to_vec()));
    let mut stdout = virtual_fs::ArcFile::new(Box::<virtual_fs::BufferFile>::default());
    let mut stderr = virtual_fs::ArcFile::new(Box::<virtual_fs::BufferFile>::default());
    let stdout_guest = stdout.clone();
    let stderr_guest = stderr.clone();
    let program = program_name.to_owned();

    let join = tokio::task::spawn_blocking(move || {
        let _guard = tasks.runtime_handle().enter();
        WasiRunner::new()
            .with_stdin(Box::new(stdin) as Box<_>)
            .with_stdout(Box::new(stdout_guest) as Box<_>)
            .with_stderr(Box::new(stderr_guest) as Box<_>)
            .run_wasm(
                RuntimeOrEngine::Engine(engine),
                &program,
                module,
                module_hash,
            )
    });

    let run_result = join.await.context("join wasmer wasi task")?;
    read_guest_output(&mut stdout, &mut stderr, run_result.is_ok()).await
}

fn ensure_webc_payload(body: &[u8]) -> Result<()> {
    if body.len() < 1024 {
        bail!(
            "Wasmer package download looks too small ({} bytes); check Accept header",
            body.len()
        );
    }
    Ok(())
}

fn build_runtime(cache_root: &Path) -> Result<(PluggableRuntime, Arc<TokioTaskManager>)> {
    let tasks = Arc::new(TokioTaskManager::new(Handle::current()));
    let mut runtime = PluggableRuntime::new(Arc::clone(&tasks) as Arc<_>);
    let compiled = cache_root.join("compiled");
    let packages = cache_root.join("packages");
    std::fs::create_dir_all(&compiled).with_context(|| format!("create {}", compiled.display()))?;
    std::fs::create_dir_all(&packages).with_context(|| format!("create {}", packages.display()))?;
    let module_cache =
        SharedCache::default().with_fallback(FileSystemCache::new(compiled, Arc::clone(&tasks)));
    runtime
        .set_module_cache(module_cache)
        .set_package_loader(BuiltinPackageLoader::new().with_cache_dir(packages));
    Ok((runtime, tasks))
}

async fn read_guest_output(
    stdout: &mut (impl AsyncReadExt + AsyncSeekExt + Unpin),
    stderr: &mut (impl AsyncReadExt + AsyncSeekExt + Unpin),
    success: bool,
) -> Result<GuestOutput> {
    stdout.rewind().await.context("rewind stdout")?;
    stderr.rewind().await.context("rewind stderr")?;
    let mut out_buf = Vec::new();
    let mut err_buf = Vec::new();
    stdout
        .read_to_end(&mut out_buf)
        .await
        .context("read stdout")?;
    stderr
        .read_to_end(&mut err_buf)
        .await
        .context("read stderr")?;
    Ok(GuestOutput {
        stdout: out_buf,
        stderr: err_buf,
        success,
    })
}
