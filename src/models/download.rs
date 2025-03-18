use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Download {
    pub id: Uuid,
    pub url: String,
    pub status: DownloadStatus,
    pub file_path: Option<String>,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DownloadStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl From<String> for DownloadStatus {
    fn from(status: String) -> Self {
        match status.to_uppercase().as_str() {
            "PENDING" => DownloadStatus::Pending,
            "IN_PROGRESS" => DownloadStatus::InProgress,
            "COMPLETED" => DownloadStatus::Completed,
            "FAILED" => DownloadStatus::Failed,
            _ => DownloadStatus::Failed,
        }
    }
}
