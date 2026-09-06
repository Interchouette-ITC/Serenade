//! Install a process-wide tracing subscriber.

use tracing::Subscriber;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

use crate::{LoggingConfig, ObservabilityError, Rotation};

/// Keeps the non-blocking file writer alive for the process lifetime.
///
/// Drop after the application finishes (or keep it in `main` until exit).
#[derive(Debug)]
pub struct LoggingGuard {
    _file_guard: Option<WorkerGuard>,
}

/// Builds a subscriber and optional file-writer guard without installing it globally.
///
/// Prefer [`init`] in application `main`. Tests can use
/// [`tracing::subscriber::with_default`] with the returned subscriber.
///
/// # Errors
///
/// Returns [`ObservabilityError`] when configuration is invalid, the log directory
/// cannot be created, or filter directives fail to parse.
pub fn build_subscriber(
    config: &LoggingConfig,
) -> Result<(impl Subscriber + Send + Sync, LoggingGuard), ObservabilityError> {
    config.validate()?;

    let filter = build_filter(config)?;
    let stderr_layer = config.stderr.then(|| {
        fmt::layer()
            .with_ansi(true)
            .with_target(true)
            .with_writer(std::io::stderr)
    });

    let (file_layer, file_guard) = if config.file {
        std::fs::create_dir_all(&config.log_dir).map_err(|source| {
            ObservabilityError::CreateDir {
                path: config.log_dir.display().to_string(),
                source,
            }
        })?;
        let appender = match config.rotation {
            Rotation::Never => rolling::never(&config.log_dir, config.log_file_name()),
            Rotation::Daily => rolling::daily(&config.log_dir, config.log_file_name()),
        };
        let (writer, guard) = tracing_appender::non_blocking(appender);
        let layer = fmt::layer()
            .with_ansi(false)
            .with_target(true)
            .with_writer(writer);
        (Some(layer), Some(guard))
    } else {
        (None, None)
    };

    let subscriber = Registry::default()
        .with(filter)
        .with(stderr_layer)
        .with(file_layer);
    Ok((
        subscriber,
        LoggingGuard {
            _file_guard: file_guard,
        },
    ))
}

/// Installs the global tracing subscriber for this process.
///
/// Call once from `main` before kernel boot. Keep the returned [`LoggingGuard`]
/// alive until shutdown so file logs flush.
///
/// # Errors
///
/// See [`build_subscriber`]. Also returns [`ObservabilityError::AlreadyInitialized`]
/// when a global subscriber is already set.
pub fn init(config: &LoggingConfig) -> Result<LoggingGuard, ObservabilityError> {
    let (subscriber, guard) = build_subscriber(config)?;
    subscriber
        .try_init()
        .map_err(|_| ObservabilityError::AlreadyInitialized)?;
    Ok(guard)
}

fn build_filter(config: &LoggingConfig) -> Result<EnvFilter, ObservabilityError> {
    config.resolve_filter_directives().map_or_else(
        || Ok(EnvFilter::default().add_directive(config.default_level.into())),
        |directives| {
            EnvFilter::try_new(directives)
                .map_err(|error| ObservabilityError::InvalidFilter(error.to_string()))
        },
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::thread;
    use std::time::Duration;

    use serenade_kernel::Environment;
    use tracing::subscriber::with_default;

    use super::*;
    use crate::{channels, LoggingConfig, Rotation};

    #[test]
    fn file_sink_writes_channel_line() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = LoggingConfig::for_environment(&Environment::Test, dir.path())
            .with_stderr(false)
            .with_rotation(Rotation::Never);
        let file_name = config.log_file_name();
        let (subscriber, guard) = build_subscriber(&config).expect("subscriber");
        with_default(subscriber, || {
            tracing::info!(target: channels::APP, "observability-smoke");
        });
        drop(guard);
        thread::sleep(Duration::from_millis(50));
        let path = dir.path().join(file_name);
        let contents = fs::read_to_string(&path).expect("read log");
        assert!(
            contents.contains("observability-smoke"),
            "log missing payload: {contents}"
        );
        assert!(
            contents.contains(channels::APP) || contents.contains("serenade::app"),
            "log missing channel: {contents}"
        );
    }

    #[test]
    fn invalid_filter_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = LoggingConfig::for_environment(&Environment::Dev, dir.path())
            .with_filter_directives("%%%");
        match build_subscriber(&config) {
            Err(ObservabilityError::InvalidFilter(_)) => {}
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("expected invalid filter"),
        }
    }

    #[test]
    fn stderr_only_builds() {
        let config = LoggingConfig::for_environment(&Environment::Prod, "unused")
            .with_file(false)
            .with_stderr(true);
        let (_subscriber, _guard) = build_subscriber(&config).expect("stderr only");
    }
}
