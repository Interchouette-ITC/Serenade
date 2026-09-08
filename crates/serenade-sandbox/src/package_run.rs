//! Arguments for [`crate::run_package`].

use std::path::Path;

/// Inputs for running a Wasmer registry package command.
pub struct PackageRun<'a> {
    /// Bytes fed to guest stdin.
    pub stdin_bytes: &'a [u8],
    /// Package/module cache root.
    pub cache_root: &'a Path,
    /// Registry URL (Accept: application/webc).
    pub package_url: &'a str,
    /// Filename under `downloads/` for the cached webc.
    pub cache_name: &'a str,
    /// Command name inside the package.
    pub command: &'a str,
    /// Arguments passed to the command.
    pub args: Vec<String>,
}
