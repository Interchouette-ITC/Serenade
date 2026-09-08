//! Guest stdout/stderr capture and JSON decode.

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;

/// Captured guest stdio after a WASIX run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestOutput {
    /// Bytes written to stdout.
    pub stdout: Vec<u8>,
    /// Bytes written to stderr.
    pub stderr: Vec<u8>,
    /// Whether the guest exited successfully.
    pub success: bool,
}

/// Decodes JSON from guest stdout when the run succeeded.
///
/// # Errors
///
/// Returns an error when the guest failed, stdout is empty with stderr present,
/// or stdout is not valid JSON for `T`.
pub fn decode_json<T: DeserializeOwned>(output: &GuestOutput) -> Result<T> {
    if !output.success {
        bail!(
            "Wasmer guest failed; stderr: {}; stdout: {}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }

    if output.stdout.is_empty() && !output.stderr.is_empty() {
        bail!(
            "guest produced no stdout; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    serde_json::from_slice(&output.stdout).with_context(|| {
        format!(
            "parse guest JSON from stdout `{}` (stderr: {})",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}
