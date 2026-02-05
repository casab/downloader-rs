use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

/// Database row representation of a download (for sqlx::FromRow).
#[derive(Debug, sqlx::FromRow)]
pub struct DownloadRow {
    pub id: Uuid,
    pub url: String,
    pub status: DownloadStatus,
    pub file_path: Option<String>,
    pub user_id: Uuid,
    pub bytes_downloaded: Option<i64>,
    pub total_bytes: Option<i64>,
    pub content_type: Option<String>,
    pub filename: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: Option<i32>,
    pub max_retries: Option<i32>,
    pub priority: Option<i32>,
    pub started_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<DownloadRow> for Download {
    fn from(row: DownloadRow) -> Self {
        Self {
            id: row.id,
            url: row.url,
            status: row.status,
            file_path: row.file_path,
            user_id: row.user_id,
            bytes_downloaded: row.bytes_downloaded.unwrap_or(0),
            total_bytes: row.total_bytes,
            content_type: row.content_type,
            filename: row.filename,
            error_message: row.error_message,
            retry_count: row.retry_count.unwrap_or(0),
            max_retries: row.max_retries.unwrap_or(3),
            priority: row.priority.unwrap_or(0),
            started_at: row.started_at,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
            completed_at: row.completed_at,
        }
    }
}

/// Full download record.
#[derive(Debug, Serialize, Deserialize)]
pub struct Download {
    pub id: Uuid,
    pub url: String,
    pub status: DownloadStatus,
    pub file_path: Option<String>,
    pub user_id: Uuid,
    /// Number of bytes downloaded so far.
    pub bytes_downloaded: i64,
    /// Total size in bytes (None if unknown).
    pub total_bytes: Option<i64>,
    /// MIME type of the downloaded file.
    pub content_type: Option<String>,
    /// Original filename from Content-Disposition or URL.
    pub filename: Option<String>,
    /// Last error message if failed.
    pub error_message: Option<String>,
    /// Number of retry attempts made.
    pub retry_count: i32,
    /// Maximum retry attempts allowed.
    pub max_retries: i32,
    /// Download priority (higher = more priority).
    pub priority: i32,
    /// When the download actually started.
    pub started_at: Option<DateTime<Utc>>,
    /// Additional metadata as JSON.
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Download {
    /// Calculate download progress percentage.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn progress_percentage(&self) -> Option<f32> {
        self.total_bytes.map(|total| {
            if total > 0 {
                (self.bytes_downloaded as f32 / total as f32) * 100.0
            } else {
                0.0
            }
        })
    }

    /// Check if the download can be paused.
    #[must_use]
    pub fn can_pause(&self) -> bool {
        self.status == DownloadStatus::InProgress
    }

    /// Check if the download can be resumed.
    #[must_use]
    pub fn can_resume(&self) -> bool {
        self.status == DownloadStatus::Paused
    }

    /// Check if the download can be retried.
    #[must_use]
    pub fn can_retry(&self) -> bool {
        self.status == DownloadStatus::Failed && self.retry_count < self.max_retries
    }

    /// Check if the download can be cancelled.
    #[must_use]
    pub fn can_cancel(&self) -> bool {
        matches!(
            self.status,
            DownloadStatus::Pending | DownloadStatus::InProgress | DownloadStatus::Paused
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DownloadStatus {
    Pending,
    InProgress,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadStatus {
    /// Check if a state transition is valid.
    #[must_use]
    pub fn can_transition_to(&self, target: &DownloadStatus) -> bool {
        use DownloadStatus::{Cancelled, Completed, Failed, InProgress, Paused, Pending};
        matches!(
            (self, target),
            // From Pending
            (Pending | Paused, InProgress)
                | (Pending | InProgress | Paused, Cancelled)
                | (InProgress, Paused | Completed | Failed)
                | (Failed, Pending)
        )
    }

    /// Get a human-readable description of the status.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Pending => "Waiting to start",
            Self::InProgress => "Downloading",
            Self::Paused => "Paused",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }

    /// Check if the download is in a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Check if the download is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Pending | Self::InProgress | Self::Paused)
    }
}

impl std::fmt::Display for DownloadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::InProgress => write!(f, "IN_PROGRESS"),
            Self::Paused => write!(f, "PAUSED"),
            Self::Completed => write!(f, "COMPLETED"),
            Self::Failed => write!(f, "FAILED"),
            Self::Cancelled => write!(f, "CANCELLED"),
        }
    }
}

impl From<String> for DownloadStatus {
    fn from(status: String) -> Self {
        match status.to_uppercase().as_str() {
            "PENDING" => DownloadStatus::Pending,
            "IN_PROGRESS" => DownloadStatus::InProgress,
            "PAUSED" => DownloadStatus::Paused,
            "COMPLETED" => DownloadStatus::Completed,
            "CANCELLED" => DownloadStatus::Cancelled,
            // "FAILED" and any unrecognized status default to Failed
            _ => DownloadStatus::Failed,
        }
    }
}

/// Download progress information for API responses.
#[derive(Debug, Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct DownloadProgress {
    pub id: Uuid,
    pub status: DownloadStatus,
    pub bytes_downloaded: i64,
    pub total_bytes: Option<i64>,
    pub percentage: Option<f32>,
    pub speed_bytes_per_sec: Option<i64>,
    pub eta_seconds: Option<i64>,
    pub started_at: Option<DateTime<Utc>>,
    pub elapsed_seconds: Option<i64>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_cancel: bool,
    pub can_retry: bool,
}

impl DownloadProgress {
    /// Create progress info from a download.
    #[must_use]
    pub fn from_download(download: &Download) -> Self {
        let elapsed_seconds = download
            .started_at
            .map(|started| (Utc::now() - started).num_seconds());

        let speed_bytes_per_sec = elapsed_seconds.and_then(|elapsed| {
            if elapsed > 0 && download.bytes_downloaded > 0 {
                Some(download.bytes_downloaded / elapsed)
            } else {
                None
            }
        });

        let eta_seconds = speed_bytes_per_sec.and_then(|speed| {
            download.total_bytes.and_then(|total| {
                let remaining = total - download.bytes_downloaded;
                if speed > 0 && remaining > 0 {
                    Some(remaining / speed)
                } else {
                    None
                }
            })
        });

        Self {
            id: download.id,
            status: download.status,
            bytes_downloaded: download.bytes_downloaded,
            total_bytes: download.total_bytes,
            percentage: download.progress_percentage(),
            speed_bytes_per_sec,
            eta_seconds,
            started_at: download.started_at,
            elapsed_seconds,
            error_message: download.error_message.clone(),
            retry_count: download.retry_count,
            can_pause: download.can_pause(),
            can_resume: download.can_resume(),
            can_cancel: download.can_cancel(),
            can_retry: download.can_retry(),
        }
    }
}

/// Request to create a new download.
#[derive(Debug, Deserialize)]
pub struct CreateDownloadRequest {
    pub url: String,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub max_retries: Option<i32>,
}

/// Retry policy configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Initial delay in milliseconds before first retry.
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds.
    pub max_delay_ms: u64,
    /// Base for exponential backoff.
    pub exponential_base: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
            exponential_base: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Calculate delay for a given retry attempt.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    pub fn delay_for_attempt(&self, attempt: u32) -> std::time::Duration {
        let delay = self.initial_delay_ms as f64 * self.exponential_base.powi(attempt as i32);
        std::time::Duration::from_millis(delay.min(self.max_delay_ms as f64) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_transitions() {
        use DownloadStatus::*;

        // Valid transitions
        assert!(Pending.can_transition_to(&InProgress));
        assert!(InProgress.can_transition_to(&Paused));
        assert!(InProgress.can_transition_to(&Completed));
        assert!(InProgress.can_transition_to(&Failed));
        assert!(Paused.can_transition_to(&InProgress));
        assert!(Failed.can_transition_to(&Pending));

        // Invalid transitions
        assert!(!Completed.can_transition_to(&InProgress));
        assert!(!Cancelled.can_transition_to(&InProgress));
        assert!(!Pending.can_transition_to(&Completed));
    }

    #[test]
    fn test_status_is_terminal() {
        assert!(DownloadStatus::Completed.is_terminal());
        assert!(DownloadStatus::Failed.is_terminal());
        assert!(DownloadStatus::Cancelled.is_terminal());
        assert!(!DownloadStatus::Pending.is_terminal());
        assert!(!DownloadStatus::InProgress.is_terminal());
    }

    #[test]
    fn test_retry_policy_delay() {
        let policy = RetryPolicy::default();

        // First retry: 1000ms
        assert_eq!(policy.delay_for_attempt(0).as_millis(), 1000);
        // Second retry: 2000ms
        assert_eq!(policy.delay_for_attempt(1).as_millis(), 2000);
        // Third retry: 4000ms
        assert_eq!(policy.delay_for_attempt(2).as_millis(), 4000);
    }

    #[test]
    fn test_retry_policy_max_delay() {
        let policy = RetryPolicy {
            max_delay_ms: 5000,
            ..Default::default()
        };

        // Should cap at max_delay_ms
        assert!(policy.delay_for_attempt(10).as_millis() <= 5000);
    }
}
