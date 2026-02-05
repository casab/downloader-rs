//! Structured event types for the domain event system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A structured domain event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub metadata: EventMetadata,
}

/// Metadata attached to every event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub user_id: Option<Uuid>,
    pub correlation_id: Option<String>,
}

impl Event {
    /// Create a new event with data serialized to JSON.
    pub fn new<T: Serialize>(event_type: &str, data: T) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: event_type.to_string(),
            source: "downloader-rs".to_string(),
            timestamp: Utc::now(),
            data: serde_json::to_value(data).unwrap_or_default(),
            metadata: EventMetadata {
                trace_id: None,
                span_id: None,
                user_id: None,
                correlation_id: None,
            },
        }
    }

    /// Attach a user ID to the event.
    #[must_use]
    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.metadata.user_id = Some(user_id);
        self
    }

    /// Attach a correlation ID to the event.
    #[must_use]
    pub fn with_correlation_id(mut self, id: String) -> Self {
        self.metadata.correlation_id = Some(id);
        self
    }

    /// Get the topic for this event type.
    #[must_use]
    pub fn topic(&self) -> &str {
        if self.event_type.starts_with("download.") {
            "downloader.downloads"
        } else if self.event_type.starts_with("user.") {
            "downloader.users"
        } else if self.event_type.starts_with("job.") {
            "downloader.jobs"
        } else {
            "downloader.system"
        }
    }
}

// ── Download Events ──

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadStarted {
    pub download_id: Uuid,
    pub url: String,
    pub user_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadCompleted {
    pub download_id: Uuid,
    pub url: String,
    pub user_id: Uuid,
    pub file_path: String,
    pub bytes: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadFailed {
    pub download_id: Uuid,
    pub url: String,
    pub user_id: Uuid,
    pub error: String,
    pub attempts: i32,
}

// ── User Events ──

#[derive(Debug, Serialize, Deserialize)]
pub struct UserRegistered {
    pub user_id: Uuid,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLoggedIn {
    pub user_id: Uuid,
    pub method: String,
}

// ── Job Events ──

#[derive(Debug, Serialize, Deserialize)]
pub struct JobEnqueued {
    pub job_id: Uuid,
    pub job_type: String,
    pub priority: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobCompleted {
    pub job_id: Uuid,
    pub job_type: String,
    pub duration_ms: i64,
}

// ── Admin Events ──

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminAction {
    pub admin_id: Uuid,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
}

/// Kafka producer configuration.
#[derive(Debug, Clone, Deserialize)]
#[derive(Default)]
pub struct EventsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub kafka: KafkaConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    #[serde(default = "default_brokers")]
    pub brokers: Vec<String>,
    #[serde(default = "default_client_id")]
    pub client_id: String,
    #[serde(default)]
    pub producer: KafkaProducerConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaProducerConfig {
    #[serde(default = "default_acks")]
    pub acks: String,
    #[serde(default = "default_retries")]
    pub retries: u32,
    #[serde(default = "default_linger_ms")]
    pub linger_ms: u32,
    #[serde(default = "default_batch_size")]
    pub batch_size: u32,
}

fn default_brokers() -> Vec<String> {
    vec!["localhost:9092".to_string()]
}
fn default_client_id() -> String {
    "downloader-rs".to_string()
}
fn default_acks() -> String {
    "all".to_string()
}
fn default_retries() -> u32 {
    3
}
fn default_linger_ms() -> u32 {
    5
}
fn default_batch_size() -> u32 {
    16384
}


impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            brokers: default_brokers(),
            client_id: default_client_id(),
            producer: KafkaProducerConfig::default(),
        }
    }
}

impl Default for KafkaProducerConfig {
    fn default() -> Self {
        Self {
            acks: default_acks(),
            retries: default_retries(),
            linger_ms: default_linger_ms(),
            batch_size: default_batch_size(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new("download.started", DownloadStarted {
            download_id: Uuid::new_v4(),
            url: "https://example.com/file.zip".to_string(),
            user_id: Uuid::new_v4(),
        });

        assert_eq!(event.event_type, "download.started");
        assert_eq!(event.source, "downloader-rs");
        assert!(event.metadata.user_id.is_none());
    }

    #[test]
    fn test_event_with_user() {
        let user_id = Uuid::new_v4();
        let event = Event::new("user.registered", UserRegistered {
            user_id,
            email: "test@example.com".to_string(),
        })
        .with_user(user_id);

        assert_eq!(event.metadata.user_id, Some(user_id));
    }

    #[test]
    fn test_event_topic_routing() {
        let download_event = Event::new("download.started", serde_json::json!({}));
        assert_eq!(download_event.topic(), "downloader.downloads");

        let user_event = Event::new("user.registered", serde_json::json!({}));
        assert_eq!(user_event.topic(), "downloader.users");

        let job_event = Event::new("job.completed", serde_json::json!({}));
        assert_eq!(job_event.topic(), "downloader.jobs");

        let system_event = Event::new("system.startup", serde_json::json!({}));
        assert_eq!(system_event.topic(), "downloader.system");
    }

    #[test]
    fn test_event_with_correlation_id() {
        let event = Event::new("download.started", serde_json::json!({}))
            .with_correlation_id("req-12345".to_string());

        assert_eq!(event.metadata.correlation_id, Some("req-12345".to_string()));
    }

    #[test]
    fn test_events_config_default() {
        let config = EventsConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.kafka.brokers, vec!["localhost:9092"]);
        assert_eq!(config.kafka.client_id, "downloader-rs");
        assert_eq!(config.kafka.producer.retries, 3);
    }

    #[test]
    fn test_event_serialization() {
        let event = Event::new("test.event", serde_json::json!({"key": "value"}));
        let json = serde_json::to_string(&event);
        assert!(json.is_ok());

        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("test.event"));
        assert!(json_str.contains("downloader-rs"));
    }
}
