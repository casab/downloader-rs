use crate::{
    middlewares::UserId,
    models::{Download, DownloadStatus},
};
use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

#[tracing::instrument(name = "Get a download by id from database", skip(pool))]
pub async fn get_download_by_id(download_id: Uuid, pool: &PgPool) -> Result<Option<Download>> {
    let download = sqlx::query_as!(
        Download,
        "SELECT * FROM downloads WHERE id = $1",
        download_id
    )
    .fetch_optional(pool)
    .await
    .context("Failed to fetch the download with given id")?;
    Ok(download)
}

#[tracing::instrument(name = "Insert a download in database", skip(pool))]
pub async fn create_download(url: &str, user_id: &UserId, pool: &PgPool) -> Result<Download> {
    let download = sqlx::query_as!(
        Download,
        "INSERT INTO downloads (id, url, status, user_id) VALUES ($1, $2, $3, $4) returning *",
        Uuid::new_v4(),
        url,
        DownloadStatus::Pending as DownloadStatus,
        user_id.0
    )
    .fetch_one(pool)
    .await
    .context("Failed to insert a download")?;
    Ok(download)
}

#[tracing::instrument(name = "Update download status", skip(pool))]
pub async fn update_download_status(
    download_id: Uuid,
    status: DownloadStatus,
    file_path: Option<String>,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let query = match status {
        DownloadStatus::Completed => {
            sqlx::query!(
                "UPDATE downloads SET status = $1, completed_at = NOW(), updated_at = NOW(), file_path = $3 WHERE id = $2",
                status as DownloadStatus,
                download_id,
                file_path
            )
        }
        _ => sqlx::query!(
            "UPDATE downloads SET status = $1, updated_at = NOW() WHERE id = $2",
            status as DownloadStatus,
            download_id
        ),
    };
    query
        .execute(pool)
        .await
        .context("Failed to update the download status")?;
    Ok(())
}

#[tracing::instrument(name = "Get all downloads from database", skip(pool))]
pub async fn get_all_downloads(
    pool: &PgPool,
    user_id: &UserId,
) -> Result<Vec<Download>, anyhow::Error> {
    let downloads = sqlx::query_as!(
        Download,
        "SELECT * FROM downloads WHERE user_id = $1",
        user_id.0
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch all downloads")?;
    Ok(downloads)
}
