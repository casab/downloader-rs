use crate::models::{Download, DownloadStatus};
use anyhow::Context;
use sqlx::PgPool;

#[tracing::instrument(name = "Get a download by id from database", skip(pool))]
pub async fn get_download_by_id(id: i64, pool: &PgPool) -> Result<Download, anyhow::Error> {
    let download = sqlx::query_as!(Download, "SELECT * FROM downloads WHERE id = $1", id)
        .fetch_one(pool)
        .await
        .context("Failed to fetch the download with given id")?;
    Ok(download)
}

#[tracing::instrument(name = "Insert a download in database", skip(pool))]
pub async fn create_download(url: &str, pool: &PgPool) -> Result<Download, anyhow::Error> {
    let download = sqlx::query_as!(
        Download,
        "INSERT INTO downloads (url, status) VALUES ($1, $2) returning *",
        url,
        DownloadStatus::Pending as DownloadStatus,
    )
    .fetch_one(pool)
    .await
    .context("Failed to insert a download")?;
    Ok(download)
}

#[tracing::instrument(name = "Update download status", skip(pool))]
pub async fn update_download_status(
    id: i64,
    status: DownloadStatus,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let query = match status {
        DownloadStatus::Completed => {
            sqlx::query!(
                "UPDATE downloads SET status = $1, completed_at = NOW(), updated_at = NOW() WHERE id = $2",
                status as DownloadStatus,
                id
            )
        }
        _ => sqlx::query!(
            "UPDATE downloads SET status = $1, updated_at = NOW() WHERE id = $2",
            status as DownloadStatus,
            id
        ),
    };
    query
        .execute(pool)
        .await
        .context("Failed to update the download status")?;
    Ok(())
}
