//! Process builder and runner.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::ProcessError;
use crate::output::CompletedProcess;

/// Symfony-shaped child process builder.
#[derive(Debug, Clone)]
pub struct Process {
    command: OsString,
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
    env: Vec<(OsString, OsString)>,
    clear_env: bool,
    timeout: Option<Duration>,
}

impl Process {
    /// Create a process for `command` (no args yet).
    #[must_use]
    pub fn new(command: impl AsRef<OsStr>) -> Self {
        Self {
            command: command.as_ref().to_owned(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            clear_env: false,
            timeout: None,
        }
    }

    /// Append one argument.
    #[must_use]
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.args.push(arg.as_ref().to_owned());
        self
    }

    /// Append several arguments.
    #[must_use]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|s| s.as_ref().to_owned()));
        self
    }

    /// Set working directory.
    #[must_use]
    pub fn cwd(mut self, path: impl AsRef<Path>) -> Self {
        self.cwd = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set an environment variable for the child.
    #[must_use]
    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.env
            .push((key.as_ref().to_owned(), value.as_ref().to_owned()));
        self
    }

    /// Do not inherit the parent environment (only explicit [`Self::env`] entries).
    #[must_use]
    pub const fn clear_env(mut self) -> Self {
        self.clear_env = true;
        self
    }

    /// Kill the child if it runs longer than `timeout`.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Command name as display string.
    #[must_use]
    pub fn command_string(&self) -> String {
        self.command.to_string_lossy().into_owned()
    }

    /// Run and capture stdout/stderr. Non-zero exit is still `Ok` (inspect status).
    ///
    /// # Errors
    ///
    /// Empty command, invalid cwd, spawn/wait I/O, or timeout.
    pub fn run(&self) -> Result<CompletedProcess, ProcessError> {
        if self.command.is_empty() {
            return Err(ProcessError::EmptyCommand);
        }
        if let Some(cwd) = &self.cwd {
            if !cwd.is_dir() {
                return Err(ProcessError::InvalidCwd { path: cwd.clone() });
            }
        }

        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }
        if self.clear_env {
            cmd.env_clear();
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn().map_err(|source| ProcessError::Io {
            command: self.command_string(),
            source,
        })?;

        let mut stdout_pipe = child.stdout.take();
        let mut stderr_pipe = child.stderr.take();

        let stdout_handle = thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut pipe) = stdout_pipe.take() {
                let _ = pipe.read_to_end(&mut buf);
            }
            buf
        });
        let stderr_handle = thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut pipe) = stderr_pipe.take() {
                let _ = pipe.read_to_end(&mut buf);
            }
            buf
        });

        let status = match self.timeout {
            None => child.wait().map_err(|source| ProcessError::Io {
                command: self.command_string(),
                source,
            })?,
            Some(timeout) => wait_with_timeout(&mut child, timeout, &self.command_string())?,
        };

        let stdout = stdout_handle.join().unwrap_or_default();
        let stderr = stderr_handle.join().unwrap_or_default();
        Ok(CompletedProcess::new(status, stdout, stderr))
    }

    /// Like [`Self::run`], but returns [`ProcessError::Failed`] on non-zero exit.
    ///
    /// # Errors
    ///
    /// Same as [`Self::run`], plus failed exit status.
    pub fn must_run(&self) -> Result<CompletedProcess, ProcessError> {
        let completed = self.run()?;
        if completed.is_successful() {
            Ok(completed)
        } else {
            Err(ProcessError::Failed {
                command: self.command_string(),
                code: completed.code(),
            })
        }
    }
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
    command: &str,
) -> Result<std::process::ExitStatus, ProcessError> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ProcessError::TimedOut {
                        command: command.to_owned(),
                        timeout,
                    });
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(source) => {
                return Err(ProcessError::Io {
                    command: command.to_owned(),
                    source,
                });
            }
        }
    }
}
