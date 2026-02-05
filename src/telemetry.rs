use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    Resource,
    metrics::SdkMeterProvider,
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use serde::Deserialize;
use tokio::task::JoinHandle;
use tracing::{Subscriber, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

/// Telemetry configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TelemetryConfig {
    #[serde(default = "default_service_name")]
    pub service_name: String,
    #[serde(default = "default_service_version")]
    pub service_version: String,
    #[serde(default)]
    pub otlp: OtlpConfig,
    #[serde(default)]
    pub tracing: TracingConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OtlpConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_otlp_endpoint")]
    pub endpoint: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TracingConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_export_interval")]
    pub export_interval_seconds: u64,
}

fn default_service_name() -> String {
    "downloader-rs".to_string()
}
fn default_service_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
fn default_otlp_endpoint() -> String {
    "http://localhost:4317".to_string()
}
fn default_true() -> bool {
    true
}
fn default_sample_rate() -> f64 {
    1.0
}
fn default_export_interval() -> u64 {
    60
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: default_service_name(),
            service_version: default_service_version(),
            otlp: OtlpConfig::default(),
            tracing: TracingConfig::default(),
            metrics: MetricsConfig::default(),
        }
    }
}

impl Default for OtlpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: default_otlp_endpoint(),
        }
    }
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sample_rate: default_sample_rate(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            export_interval_seconds: default_export_interval(),
        }
    }
}

/// Build the base tracing subscriber (no OpenTelemetry).
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

#[allow(clippy::expect_used)]
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger.");
    set_global_default(subscriber).expect("Failed to set subscriber.");
}

/// Initialize OpenTelemetry tracing and metrics when OTLP is enabled.
/// Returns a guard that should be kept alive for the lifetime of the application.
pub fn init_opentelemetry(config: &TelemetryConfig) -> Option<OtelGuard> {
    if !config.otlp.enabled {
        return None;
    }

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new(SERVICE_NAME, config.service_name.clone()),
            KeyValue::new(SERVICE_VERSION, config.service_version.clone()),
        ])
        .build();

    // Set up tracer provider
    let span_exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&config.otlp.endpoint)
        .build()
        .ok()?;

    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(span_exporter)
        .with_sampler(Sampler::TraceIdRatioBased(config.tracing.sample_rate))
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource.clone())
        .build();

    // Set up meter provider
    let metric_exporter = MetricExporter::builder()
        .with_tonic()
        .with_endpoint(&config.otlp.endpoint)
        .build()
        .ok()?;

    let meter_provider = SdkMeterProvider::builder()
        .with_periodic_exporter(metric_exporter)
        .with_resource(resource)
        .build();

    opentelemetry::global::set_meter_provider(meter_provider.clone());

    Some(OtelGuard {
        tracer_provider,
        meter_provider,
    })
}

/// Build a subscriber with OpenTelemetry layer attached.
pub fn get_subscriber_with_otel<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
    otel_guard: &OtelGuard,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);

    let tracer = otel_guard.tracer_provider.tracer("downloader-rs");
    let otel_layer = OpenTelemetryLayer::new(tracer);

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
        .with(otel_layer)
}

/// Guard that keeps OpenTelemetry providers alive and shuts them down on drop.
pub struct OtelGuard {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(e) = self.tracer_provider.shutdown() {
            tracing::warn!("Failed to shut down tracer provider: {e}");
        }
        if let Err(e) = self.meter_provider.shutdown() {
            tracing::warn!("Failed to shut down meter provider: {e}");
        }
    }
}

pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert_eq!(config.service_name, "downloader-rs");
        assert!(!config.otlp.enabled);
        assert!(config.tracing.enabled);
        assert!((config.tracing.sample_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_otlp_config_default() {
        let config = OtlpConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.endpoint, "http://localhost:4317");
    }

    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.export_interval_seconds, 60);
    }
}
