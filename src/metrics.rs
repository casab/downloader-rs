//! Application metrics using OpenTelemetry and Prometheus client.

use opentelemetry::{
    global,
    metrics::{Counter, Histogram, UpDownCounter},
    KeyValue,
};
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application-wide metrics.
pub struct AppMetrics {
    /// Total HTTP requests.
    pub http_requests_total: Counter<u64>,
    /// HTTP request duration in seconds.
    pub http_request_duration: Histogram<f64>,
    /// Total downloads initiated.
    pub downloads_total: Counter<u64>,
    /// Downloads currently in progress.
    pub downloads_in_progress: UpDownCounter<i64>,
    /// Total bytes downloaded.
    pub downloads_bytes_total: Counter<u64>,
    /// Jobs waiting in queue.
    pub job_queue_depth: UpDownCounter<i64>,
    /// Job processing duration in seconds.
    pub job_processing_duration: Histogram<f64>,
    /// Currently active users.
    pub active_users: UpDownCounter<i64>,
    /// Prometheus registry for /metrics endpoint.
    pub prometheus_registry: Arc<RwLock<Registry>>,
}

impl AppMetrics {
    /// Create and register all application metrics.
    #[must_use]
    pub fn new() -> Self {
        let meter = global::meter("downloader-rs");

        Self {
            http_requests_total: meter
                .u64_counter("http_requests_total")
                .with_description("Total number of HTTP requests")
                .build(),

            http_request_duration: meter
                .f64_histogram("http_request_duration_seconds")
                .with_description("HTTP request duration in seconds")
                .build(),

            downloads_total: meter
                .u64_counter("downloads_total")
                .with_description("Total number of downloads initiated")
                .build(),

            downloads_in_progress: meter
                .i64_up_down_counter("downloads_in_progress")
                .with_description("Number of downloads currently in progress")
                .build(),

            downloads_bytes_total: meter
                .u64_counter("downloads_bytes_total")
                .with_description("Total bytes downloaded")
                .build(),

            job_queue_depth: meter
                .i64_up_down_counter("job_queue_depth")
                .with_description("Number of jobs waiting in queue")
                .build(),

            job_processing_duration: meter
                .f64_histogram("job_processing_duration_seconds")
                .with_description("Job processing duration in seconds")
                .build(),

            active_users: meter
                .i64_up_down_counter("active_users")
                .with_description("Number of currently active users")
                .build(),

            prometheus_registry: Arc::new(RwLock::new(Registry::default())),
        }
    }

    /// Record an HTTP request.
    pub fn record_http_request(&self, method: &str, path: &str, status: u16, duration_secs: f64) {
        let attrs = [
            KeyValue::new("method", method.to_string()),
            KeyValue::new("path", path.to_string()),
            KeyValue::new("status", i64::from(status)),
        ];
        self.http_requests_total.add(1, &attrs);
        self.http_request_duration.record(duration_secs, &attrs);
    }

    /// Record a download started.
    pub fn record_download_started(&self) {
        self.downloads_total.add(1, &[]);
        self.downloads_in_progress.add(1, &[]);
    }

    /// Record a download completed.
    pub fn record_download_completed(&self, bytes: u64) {
        self.downloads_in_progress.add(-1, &[]);
        self.downloads_bytes_total
            .add(bytes, &[KeyValue::new("status", "completed")]);
    }

    /// Record a download failed.
    pub fn record_download_failed(&self) {
        self.downloads_in_progress.add(-1, &[]);
    }

    /// Record a job enqueued.
    pub fn record_job_enqueued(&self) {
        self.job_queue_depth.add(1, &[]);
    }

    /// Record a job completed.
    pub fn record_job_completed(&self, duration_secs: f64, job_type: &str) {
        self.job_queue_depth.add(-1, &[]);
        self.job_processing_duration
            .record(duration_secs, &[KeyValue::new("job_type", job_type.to_string())]);
    }
}

impl Default for AppMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode Prometheus metrics to text format for the /metrics endpoint.
pub fn encode_prometheus_metrics(registry: &Registry) -> String {
    let mut output = String::new();
    if encode(&mut output, registry).is_err() {
        return String::new();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_metrics_creation() {
        let metrics = AppMetrics::new();
        // Just verify creation doesn't panic
        metrics.record_http_request("GET", "/api/v1/health", 200, 0.05);
    }

    #[test]
    fn test_record_download_lifecycle() {
        let metrics = AppMetrics::new();
        metrics.record_download_started();
        metrics.record_download_completed(1024);
    }

    #[test]
    fn test_record_download_failed() {
        let metrics = AppMetrics::new();
        metrics.record_download_started();
        metrics.record_download_failed();
    }

    #[test]
    fn test_record_job_lifecycle() {
        let metrics = AppMetrics::new();
        metrics.record_job_enqueued();
        metrics.record_job_completed(1.5, "download");
    }

    #[test]
    fn test_encode_empty_registry() {
        let registry = Registry::default();
        let output = encode_prometheus_metrics(&registry);
        // Empty registry produces empty or minimal output
        assert!(output.is_empty() || output.starts_with('#'));
    }
}
