//! Job models and types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Types of jobs that can be processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    /// Download a file from a URL.
    Download,
    /// Send an email notification.
    Email,
    /// Cleanup temporary files or expired data.
    Cleanup,
    /// Send a notification (in-app, push, etc.).
    Notification,
    /// Call a webhook URL.
    Webhook,
}

impl JobType {
    /// Get all job types.
    pub fn all() -> &'static [JobType] {
        &[
            JobType::Download,
            JobType::Email,
            JobType::Cleanup,
            JobType::Notification,
            JobType::Webhook,
        ]
    }

    /// Get the string representation for Redis stream keys.
    pub fn as_str(&self) -> &'static str {
        match self {
            JobType::Download => "download",
            JobType::Email => "email",
            JobType::Cleanup => "cleanup",
            JobType::Notification => "notification",
            JobType::Webhook => "webhook",
        }
    }
}

impl std::fmt::Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Status of a job in the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Job is waiting to be processed.
    Pending,
    /// Job is currently being processed.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed after all retries.
    Failed,
    /// Job was cancelled.
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Payload for different job types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobPayload {
    /// Download job payload.
    Download(DownloadPayload),
    /// Email job payload.
    Email(EmailPayload),
    /// Cleanup job payload.
    Cleanup(CleanupPayload),
    /// Notification job payload.
    Notification(NotificationPayload),
    /// Webhook job payload.
    Webhook(WebhookPayload),
}

/// Payload for download jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadPayload {
    /// ID of the download record.
    pub download_id: Uuid,
    /// User who initiated the download.
    pub user_id: Uuid,
    /// URL to download from.
    pub url: String,
    /// Optional filename override.
    pub filename: Option<String>,
    /// Priority of the download (higher = more important).
    pub priority: i32,
}

/// Payload for email jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailPayload {
    /// Recipient email address.
    pub to: String,
    /// Email subject.
    pub subject: String,
    /// Email body (HTML).
    pub body: String,
    /// User ID if applicable.
    pub user_id: Option<Uuid>,
}

/// Payload for cleanup jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPayload {
    /// Type of cleanup to perform.
    pub cleanup_type: String,
    /// Optional parameters.
    pub params: Option<serde_json::Value>,
}

/// Payload for notification jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    /// User to notify.
    pub user_id: Uuid,
    /// Notification title.
    pub title: String,
    /// Notification message.
    pub message: String,
    /// Notification type.
    pub notification_type: String,
    /// Additional data.
    pub data: Option<serde_json::Value>,
}

/// Payload for webhook jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// URL to call.
    pub url: String,
    /// HTTP method.
    pub method: String,
    /// Request headers.
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// Request body.
    pub body: Option<serde_json::Value>,
    /// User ID if applicable.
    pub user_id: Option<Uuid>,
}

/// A job to be processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier.
    pub id: Uuid,
    /// Type of job.
    pub job_type: JobType,
    /// Job payload containing type-specific data.
    pub payload: serde_json::Value,
    /// Job priority (higher = processed first).
    pub priority: i32,
    /// Number of processing attempts.
    pub attempts: i32,
    /// Maximum retry attempts.
    pub max_retries: i32,
    /// Redis stream key this job came from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_key: Option<String>,
    /// Redis stream message ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    /// When the job was created.
    pub created_at: DateTime<Utc>,
    /// User ID associated with this job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
}

impl Job {
    /// Create a new job.
    pub fn new(job_type: JobType, payload: impl Serialize) -> anyhow::Result<Self> {
        Ok(Self {
            id: Uuid::new_v4(),
            job_type,
            payload: serde_json::to_value(payload)?,
            priority: 0,
            attempts: 0,
            max_retries: 3,
            stream_key: None,
            message_id: None,
            created_at: Utc::now(),
            user_id: None,
        })
    }

    /// Create a new job with priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set the user ID for this job.
    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set max retries for this job.
    pub fn with_max_retries(mut self, max_retries: i32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Check if the job can be retried.
    pub fn can_retry(&self) -> bool {
        self.attempts < self.max_retries
    }

    /// Increment the attempt counter.
    pub fn increment_attempts(&mut self) {
        self.attempts += 1;
    }
}

/// Result of a successfully processed job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// Result data (job-type specific).
    pub data: serde_json::Value,
    /// When the job completed.
    pub completed_at: DateTime<Utc>,
}

impl JobResult {
    /// Create a new job result.
    pub fn new(data: impl Serialize) -> anyhow::Result<Self> {
        Ok(Self {
            data: serde_json::to_value(data)?,
            completed_at: Utc::now(),
        })
    }

    /// Create an empty job result.
    pub fn empty() -> Self {
        Self {
            data: serde_json::Value::Null,
            completed_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_type_all() {
        let all = JobType::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_job_type_as_str() {
        assert_eq!(JobType::Download.as_str(), "download");
        assert_eq!(JobType::Email.as_str(), "email");
    }

    #[test]
    fn test_job_creation() {
        let payload = DownloadPayload {
            download_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            url: "https://example.com/file.zip".to_string(),
            filename: None,
            priority: 0,
        };

        let job = Job::new(JobType::Download, &payload).unwrap();
        assert_eq!(job.job_type, JobType::Download);
        assert_eq!(job.priority, 0);
        assert_eq!(job.attempts, 0);
    }

    #[test]
    fn test_job_with_priority() {
        let payload = CleanupPayload {
            cleanup_type: "temp_files".to_string(),
            params: None,
        };

        let job = Job::new(JobType::Cleanup, &payload)
            .unwrap()
            .with_priority(10);

        assert_eq!(job.priority, 10);
    }

    #[test]
    fn test_job_can_retry() {
        let payload = CleanupPayload {
            cleanup_type: "test".to_string(),
            params: None,
        };

        let mut job = Job::new(JobType::Cleanup, &payload)
            .unwrap()
            .with_max_retries(3);

        assert!(job.can_retry());
        job.increment_attempts();
        job.increment_attempts();
        job.increment_attempts();
        assert!(!job.can_retry());
    }
}
