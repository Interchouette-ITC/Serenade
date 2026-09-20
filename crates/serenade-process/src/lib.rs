//! Child process spawn, timeout, and exit/output capture (Symfony Process shaped).
//!
//! - [`Process`] - builder: args, cwd, env, timeout
//! - [`CompletedProcess`] - status, stdout, stderr
//! - [`Process::run`] / [`Process::must_run`]
//!
//! # Examples
//!
//! ```
//! use serenade_process::Process;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let out = Process::new("echo").arg("hi").must_run()?;
//! assert!(out.is_successful());
//! assert!(out.stdout_string().contains("hi"));
//! # Ok(())
//! # }
//! ```

mod error;
mod output;
mod process;

pub use error::ProcessError;
pub use output::CompletedProcess;
pub use process::Process;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
