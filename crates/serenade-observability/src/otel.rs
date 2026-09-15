//! Optional OpenTelemetry tracing bridge (`feature = "otel"`).

use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing::Subscriber;
use tracing_appender::rolling;
use tracing_subscriber::Registry;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::init::{LoggingGuard, build_filter};
use crate::{LoggingConfig, ObservabilityError, Rotation};

/// Configuration for the optional OpenTelemetry tracing pipeline.
#[derive(Debug, Clone)]
pub struct OtelConfig {
    /// `service.name` resource attribute.
    pub service_name: String,
    /// OTLP/HTTP collector base URL (for example `http://127.0.0.1:4318`).
    ///
    /// When `None`, spans stay in-process (no network exporter) so apps can wire
    /// the tracing bridge without a collector.
    pub endpoint: Option<String>,
}

impl OtelConfig {
    /// Builds a config with `service_name` and no exporter endpoint.
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            endpoint: None,
        }
    }

    /// Sets the OTLP/HTTP endpoint (collector base URL).
    #[must_use]
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub(crate) fn validate(&self) -> Result<(), ObservabilityError> {
        if self.service_name.trim().is_empty() {
            return Err(ObservabilityError::Otel(
                "service_name must not be empty".into(),
            ));
        }
        if let Some(endpoint) = &self.endpoint
            && endpoint.trim().is_empty()
        {
            return Err(ObservabilityError::Otel(
                "endpoint must not be empty when set".into(),
            ));
        }
        Ok(())
    }
}

/// Flushes the OpenTelemetry tracer provider on drop.
#[derive(Debug)]
pub struct OtelGuard {
    provider: SdkTracerProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        let _ = self.provider.shutdown();
    }
}

/// Builds stderr/file logging plus an OpenTelemetry tracing layer without installing
/// a global subscriber.
///
/// # Errors
///
/// Returns [`ObservabilityError`] when logging or OpenTelemetry configuration is invalid.
pub fn build_subscriber_with_otel(
    logging: &LoggingConfig,
    otel: &OtelConfig,
) -> Result<(impl Subscriber + Send + Sync, LoggingGuard, OtelGuard), ObservabilityError> {
    logging.validate()?;
    otel.validate()?;

    let filter = build_filter(logging)?;
    let stderr_layer = logging.stderr.then(|| {
        fmt::layer()
            .with_ansi(true)
            .with_target(true)
            .with_writer(std::io::stderr)
    });

    let (file_layer, file_guard) = if logging.file {
        std::fs::create_dir_all(&logging.log_dir).map_err(|source| {
            ObservabilityError::CreateDir {
                path: logging.log_dir.display().to_string(),
                source,
            }
        })?;
        let appender = match logging.rotation {
            Rotation::Never => rolling::never(&logging.log_dir, logging.log_file_name()),
            Rotation::Daily => rolling::daily(&logging.log_dir, logging.log_file_name()),
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

    let provider = build_tracer_provider(otel)?;
    let tracer = provider.tracer("serenade");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let otel_guard = OtelGuard { provider };

    let subscriber = Registry::default()
        .with(filter)
        .with(stderr_layer)
        .with(file_layer)
        .with(otel_layer);

    Ok((
        subscriber,
        LoggingGuard::from_file_guard(file_guard),
        otel_guard,
    ))
}

/// Installs logging + OpenTelemetry as the process global subscriber.
///
/// Keep both returned guards until shutdown so file writers and the tracer
/// provider flush.
///
/// # Errors
///
/// See [`build_subscriber_with_otel`]. Also returns
/// [`ObservabilityError::AlreadyInitialized`] when a global subscriber exists.
pub fn init_with_otel(
    logging: &LoggingConfig,
    otel: &OtelConfig,
) -> Result<(LoggingGuard, OtelGuard), ObservabilityError> {
    let (subscriber, logging_guard, otel_guard) = build_subscriber_with_otel(logging, otel)?;
    subscriber
        .try_init()
        .map_err(|_| ObservabilityError::AlreadyInitialized)?;
    Ok((logging_guard, otel_guard))
}

fn build_tracer_provider(config: &OtelConfig) -> Result<SdkTracerProvider, ObservabilityError> {
    let resource = Resource::builder()
        .with_attributes([KeyValue::new("service.name", config.service_name.clone())])
        .build();

    match &config.endpoint {
        Some(endpoint) => {
            let exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_http()
                .with_endpoint(endpoint.clone())
                .build()
                .map_err(|error| ObservabilityError::Otel(error.to_string()))?;
            Ok(SdkTracerProvider::builder()
                .with_resource(resource)
                .with_batch_exporter(exporter)
                .build())
        }
        None => Ok(SdkTracerProvider::builder().with_resource(resource).build()),
    }
}

#[cfg(test)]
mod tests {
    use serenade_kernel::Environment;
    use tracing::subscriber::with_default;

    use super::*;
    use crate::{LoggingConfig, Rotation, channels};

    #[test]
    fn rejects_empty_service_name_and_endpoint() {
        assert!(matches!(
            OtelConfig::new("").validate(),
            Err(ObservabilityError::Otel(_))
        ));
        assert!(matches!(
            OtelConfig::new("svc").with_endpoint("  ").validate(),
            Err(ObservabilityError::Otel(_))
        ));
    }

    #[test]
    fn in_process_bridge_emits_spans_without_collector() {
        let dir = tempfile::tempdir().expect("tempdir");
        let logging = LoggingConfig::for_environment(&Environment::Test, dir.path())
            .with_stderr(false)
            .with_rotation(Rotation::Never);
        let otel = OtelConfig::new("serenade-test");
        let (subscriber, logging_guard, otel_guard) =
            build_subscriber_with_otel(&logging, &otel).expect("subscriber");
        with_default(subscriber, || {
            let span = tracing::info_span!(target: channels::APP, "otel-smoke");
            let _enter = span.enter();
            tracing::info!(target: channels::APP, "inside");
        });
        drop(otel_guard);
        drop(logging_guard);
    }

    #[test]
    fn otlp_endpoint_builds_exporter() {
        let dir = tempfile::tempdir().expect("tempdir");
        let logging = LoggingConfig::for_environment(&Environment::Test, dir.path())
            .with_file(false)
            .with_stderr(true);
        let otel = OtelConfig::new("serenade-otlp").with_endpoint("http://127.0.0.1:4318");
        let (_subscriber, _logging_guard, otel_guard) =
            build_subscriber_with_otel(&logging, &otel).expect("otlp build");
        drop(otel_guard);
    }
}
