//! Logging configuration.

use std::path::PathBuf;

use tracing_subscriber::filter::LevelFilter;

use serenade_kernel::Environment;

use crate::ObservabilityError;

/// How the file sink rotates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// Never rotate (single file; useful in tests).
    Never,
    /// New file each calendar day.
    Daily,
}

/// App-owned logging setup for [`crate::init`].
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Directory the application owns (for example `var/log`).
    pub log_dir: PathBuf,
    /// Environment name used for the log file stem (`dev` → `dev.log`).
    pub environment: String,
    /// Default level when no filter env is set.
    pub default_level: LevelFilter,
    /// Mirror events to stderr.
    pub stderr: bool,
    /// Write events to the rolling file under [`Self::log_dir`].
    pub file: bool,
    /// File rotation policy when [`Self::file`] is true.
    pub rotation: Rotation,
    /// Optional filter directives (`SERENADE_LOG` / `RUST_LOG` style).
    pub filter_directives: Option<String>,
}

impl LoggingConfig {
    /// Builds defaults for a Serenade [`Environment`].
    ///
    /// File name is `{env}.log` under `log_dir`. Debug environments default to
    /// [`LevelFilter::DEBUG`]; others to [`LevelFilter::INFO`].
    ///
    /// # Examples
    ///
    /// ```
    /// use serenade_kernel::Environment;
    /// use serenade_observability::LoggingConfig;
    ///
    /// let config = LoggingConfig::for_environment(&Environment::Dev, "var/log");
    /// assert_eq!(config.log_file_name(), "dev.log");
    /// ```
    #[must_use]
    pub fn for_environment(environment: &Environment, log_dir: impl Into<PathBuf>) -> Self {
        let name = environment.as_str().to_owned();
        let default_level = if environment.is_debug() {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        };
        Self {
            log_dir: log_dir.into(),
            environment: name,
            default_level,
            stderr: true,
            file: true,
            rotation: Rotation::Daily,
            filter_directives: None,
        }
    }

    /// Sets whether stderr is enabled.
    #[must_use]
    pub const fn with_stderr(mut self, stderr: bool) -> Self {
        self.stderr = stderr;
        self
    }

    /// Sets whether the file sink is enabled.
    #[must_use]
    pub const fn with_file(mut self, file: bool) -> Self {
        self.file = file;
        self
    }

    /// Sets file rotation.
    #[must_use]
    pub const fn with_rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }

    /// Sets an explicit filter string (takes precedence over the default level).
    #[must_use]
    pub fn with_filter_directives(mut self, directives: impl Into<String>) -> Self {
        self.filter_directives = Some(directives.into());
        self
    }

    /// Resolves filter directives from config, then `SERENADE_LOG`, then `RUST_LOG`.
    pub(crate) fn resolve_filter_directives(&self) -> Option<String> {
        if let Some(directives) = &self.filter_directives {
            return Some(directives.clone());
        }
        std::env::var("SERENADE_LOG")
            .ok()
            .or_else(|| std::env::var("RUST_LOG").ok())
    }

    /// Log file stem (`dev.log` for environment `dev`).
    #[must_use]
    pub fn log_file_name(&self) -> String {
        format!("{}.log", self.environment)
    }

    /// Ensures at least one sink is enabled.
    pub(crate) fn validate(&self) -> Result<(), ObservabilityError> {
        if !self.stderr && !self.file {
            return Err(ObservabilityError::NoSinks);
        }
        if self.environment.trim().is_empty() {
            return Err(ObservabilityError::EmptyEnvironment);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serenade_kernel::Environment;

    #[test]
    fn for_environment_picks_debug_level_in_dev() {
        let config = LoggingConfig::for_environment(&Environment::Dev, "var/log");
        assert_eq!(config.environment, "dev");
        assert_eq!(config.default_level, LevelFilter::DEBUG);
        assert_eq!(config.log_file_name(), "dev.log");
        assert!(config.stderr && config.file);
        assert_eq!(config.rotation, Rotation::Daily);
    }

    #[test]
    fn for_environment_picks_info_in_prod() {
        let config = LoggingConfig::for_environment(&Environment::Prod, "var/log");
        assert_eq!(config.default_level, LevelFilter::INFO);
        assert_eq!(config.log_file_name(), "prod.log");
    }

    #[test]
    fn validate_rejects_no_sinks_and_empty_env() {
        let mut config = LoggingConfig::for_environment(&Environment::Test, "var/log")
            .with_stderr(false)
            .with_file(false);
        assert!(matches!(
            config.validate(),
            Err(ObservabilityError::NoSinks)
        ));
        config.file = true;
        config.environment = String::new();
        assert!(matches!(
            config.validate(),
            Err(ObservabilityError::EmptyEnvironment)
        ));
    }

    #[test]
    fn builders_override_filter() {
        let config = LoggingConfig::for_environment(&Environment::Dev, "var/log")
            .with_filter_directives("serenade::app=trace")
            .with_rotation(Rotation::Never);
        assert_eq!(
            config.filter_directives.as_deref(),
            Some("serenade::app=trace")
        );
        assert_eq!(config.rotation, Rotation::Never);
        assert_eq!(
            config.resolve_filter_directives().as_deref(),
            Some("serenade::app=trace")
        );
    }

    #[test]
    fn resolve_filter_reads_serenade_log_then_rust_log() {
        use std::sync::{Mutex, OnceLock};

        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("lock");

        let config = LoggingConfig::for_environment(&Environment::Dev, "var/log");
        let prev_serenade = std::env::var("SERENADE_LOG").ok();
        let prev_rust = std::env::var("RUST_LOG").ok();
        unsafe {
            std::env::remove_var("SERENADE_LOG");
            std::env::remove_var("RUST_LOG");
        }
        assert!(config.resolve_filter_directives().is_none());

        unsafe {
            std::env::set_var("RUST_LOG", "info");
        }
        assert_eq!(config.resolve_filter_directives().as_deref(), Some("info"));

        unsafe {
            std::env::set_var("SERENADE_LOG", "serenade::app=debug");
        }
        assert_eq!(
            config.resolve_filter_directives().as_deref(),
            Some("serenade::app=debug")
        );

        unsafe {
            match prev_serenade {
                Some(value) => std::env::set_var("SERENADE_LOG", value),
                None => std::env::remove_var("SERENADE_LOG"),
            }
            match prev_rust {
                Some(value) => std::env::set_var("RUST_LOG", value),
                None => std::env::remove_var("RUST_LOG"),
            }
        }
    }
}
