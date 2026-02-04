//! Download job handler.

use crate::clients::S3Client;
use crate::jobs::{DownloadPayload, Job, JobHandler, JobResult, JobType};
use crate::models::DownloadStatus;
use crate::repository::{update_download_progress, update_download_status};
use crate::utils::download_file;
use async_trait::async_trait;
use redis::AsyncCommands;
use sqlx::PgPool;
use std::sync::Arc;

/// Handler for download jobs.
pub struct DownloadJobHandler {
    pool: PgPool,
    s3_client: Arc<S3Client>,
    redis_client: redis::Client,
}

impl DownloadJobHandler {
    /// Create a new download job handler.
    pub fn new(pool: PgPool, s3_client: Arc<S3Client>, redis_url: &str) -> anyhow::Result<Self> {
        let redis_client = redis::Client::open(redis_url)?;
        Ok(Self {
            pool,
            s3_client,
            redis_client,
        })
    }

    /// Publish progress update via Redis Pub/Sub.
    async fn publish_progress(
        &self,
        download_id: uuid::Uuid,
        bytes_downloaded: i64,
        total_bytes: Option<i64>,
    ) -> anyhow::Result<()> {
        let mut conn = self.redis_client.get_multiplexed_async_connection().await?;

        let progress = serde_json::json!({
            "download_id": download_id,
            "bytes_downloaded": bytes_downloaded,
            "total_bytes": total_bytes,
            "percentage": total_bytes.map(|t| (bytes_downloaded as f64 / t as f64 * 100.0) as i32),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let channel = format!("download:progress:{}", download_id);
        let _: () = conn.publish(channel, progress.to_string()).await?;

        Ok(())
    }

    /// Perform the actual download with progress tracking.
    async fn download_with_progress(&self, payload: &DownloadPayload) -> anyhow::Result<DownloadResult> {
        // Update status to InProgress
        update_download_status(
            payload.download_id,
            DownloadStatus::InProgress,
            None,
            None,
            &self.pool,
        )
        .await?;

        // Perform the download
        let file_path = download_file(&payload.url, Some((*self.s3_client).clone())).await?;

        // Get file size
        let metadata = tokio::fs::metadata(&file_path).await?;
        let total_bytes = metadata.len() as i64;

        // Update progress in database
        update_download_progress(payload.download_id, total_bytes, Some(total_bytes), &self.pool).await?;

        // Publish final progress
        self.publish_progress(payload.download_id, total_bytes, Some(total_bytes))
            .await
            .ok(); // Don't fail if publish fails

        Ok(DownloadResult {
            file_path,
            total_bytes,
        })
    }
}

#[async_trait]
impl JobHandler for DownloadJobHandler {
    async fn execute(&self, job: &Job) -> anyhow::Result<JobResult> {
        let payload: DownloadPayload = serde_json::from_value(job.payload.clone())?;

        tracing::info!(
            download_id = %payload.download_id,
            url = %payload.url,
            "Starting download job"
        );

        match self.download_with_progress(&payload).await {
            Ok(result) => {
                // Update download status to completed
                update_download_status(
                    payload.download_id,
                    DownloadStatus::Completed,
                    Some(result.file_path.clone()),
                    None,
                    &self.pool,
                )
                .await?;

                tracing::info!(
                    download_id = %payload.download_id,
                    file_path = %result.file_path,
                    total_bytes = result.total_bytes,
                    "Download completed"
                );

                JobResult::new(serde_json::json!({
                    "download_id": payload.download_id,
                    "file_path": result.file_path,
                    "total_bytes": result.total_bytes,
                }))
            }
            Err(e) => {
                // Update download status to failed
                update_download_status(
                    payload.download_id,
                    DownloadStatus::Failed,
                    None,
                    Some(e.to_string()),
                    &self.pool,
                )
                .await
                .ok(); // Don't fail if status update fails

                tracing::error!(
                    download_id = %payload.download_id,
                    error = %e,
                    "Download failed"
                );

                Err(e)
            }
        }
    }

    fn job_type(&self) -> JobType {
        JobType::Download
    }
}

/// Result of a successful download.
#[derive(Debug)]
struct DownloadResult {
    file_path: String,
    total_bytes: i64,
}
