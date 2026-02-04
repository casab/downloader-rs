//! Download repository for download management operations.

use crate::{
    middlewares::UserId,
    models::{
        Download, DownloadFilter, DownloadRow, DownloadStatus, PaginatedResponse, PaginationParams,
        SortParams,
        sorting::{DOWNLOAD_DEFAULT_ORDER, DOWNLOAD_DEFAULT_SORT, DOWNLOAD_SORT_FIELDS},
    },
};
use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

/// All columns for download queries.
const DOWNLOAD_COLUMNS: &str = r#"
    id, url, status, file_path, user_id, bytes_downloaded, total_bytes,
    content_type, filename, error_message, retry_count, max_retries,
    priority, started_at, metadata, created_at, updated_at, completed_at
"#;

/// Get a download by ID.
#[tracing::instrument(name = "Get download by ID", skip(pool))]
pub async fn get_download_by_id(download_id: Uuid, pool: &PgPool) -> Result<Option<Download>> {
    let query = format!(
        "SELECT {DOWNLOAD_COLUMNS} FROM downloads WHERE id = $1"
    );

    let row = sqlx::query_as::<_, DownloadRow>(&query)
        .bind(download_id)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch download by ID")?;

    Ok(row.map(Download::from))
}

/// Get a download by ID for a specific user (ownership check).
#[tracing::instrument(name = "Get user download by ID", skip(pool))]
pub async fn get_user_download_by_id(
    download_id: Uuid,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Option<Download>> {
    let query = format!(
        "SELECT {DOWNLOAD_COLUMNS} FROM downloads WHERE id = $1 AND user_id = $2"
    );

    let row = sqlx::query_as::<_, DownloadRow>(&query)
        .bind(download_id)
        .bind(user_id.0)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch user download by ID")?;

    Ok(row.map(Download::from))
}

/// Create a new download.
#[tracing::instrument(name = "Create download", skip(pool))]
pub async fn create_download(
    url: &str,
    user_id: &UserId,
    priority: Option<i32>,
    max_retries: Option<i32>,
    pool: &PgPool,
) -> Result<Download> {
    let query = format!(
        r#"
        INSERT INTO downloads (id, url, status, user_id, priority, max_retries)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING {DOWNLOAD_COLUMNS}
        "#
    );

    let row = sqlx::query_as::<_, DownloadRow>(&query)
        .bind(Uuid::new_v4())
        .bind(url)
        .bind(DownloadStatus::Pending)
        .bind(user_id.0)
        .bind(priority.unwrap_or(0))
        .bind(max_retries.unwrap_or(3))
        .fetch_one(pool)
        .await
        .context("Failed to create download")?;

    Ok(Download::from(row))
}

/// Update download status with state machine validation.
#[tracing::instrument(name = "Update download status", skip(pool))]
pub async fn update_download_status(
    download_id: Uuid,
    new_status: DownloadStatus,
    file_path: Option<String>,
    error_message: Option<String>,
    pool: &PgPool,
) -> Result<Download> {
    let query = match new_status {
        DownloadStatus::Completed => format!(
            r#"
            UPDATE downloads
            SET status = $2, file_path = $3, completed_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING {DOWNLOAD_COLUMNS}
            "#
        ),
        DownloadStatus::Failed => format!(
            r#"
            UPDATE downloads
            SET status = $2, error_message = $3, retry_count = retry_count + 1, updated_at = NOW()
            WHERE id = $1
            RETURNING {DOWNLOAD_COLUMNS}
            "#
        ),
        DownloadStatus::InProgress => format!(
            r#"
            UPDATE downloads
            SET status = $2, started_at = COALESCE(started_at, NOW()), updated_at = NOW()
            WHERE id = $1
            RETURNING {DOWNLOAD_COLUMNS}
            "#
        ),
        _ => format!(
            r#"
            UPDATE downloads
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING {DOWNLOAD_COLUMNS}
            "#
        ),
    };

    let row = match new_status {
        DownloadStatus::Completed => {
            sqlx::query_as::<_, DownloadRow>(&query)
                .bind(download_id)
                .bind(new_status)
                .bind(file_path)
                .fetch_one(pool)
                .await
        }
        DownloadStatus::Failed => {
            sqlx::query_as::<_, DownloadRow>(&query)
                .bind(download_id)
                .bind(new_status)
                .bind(error_message)
                .fetch_one(pool)
                .await
        }
        _ => {
            sqlx::query_as::<_, DownloadRow>(&query)
                .bind(download_id)
                .bind(new_status)
                .fetch_one(pool)
                .await
        }
    }
    .context("Failed to update download status")?;

    Ok(Download::from(row))
}

/// Update download progress.
#[tracing::instrument(name = "Update download progress", skip(pool))]
pub async fn update_download_progress(
    download_id: Uuid,
    bytes_downloaded: i64,
    total_bytes: Option<i64>,
    pool: &PgPool,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE downloads
        SET bytes_downloaded = $2, total_bytes = COALESCE($3, total_bytes), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(download_id)
    .bind(bytes_downloaded)
    .bind(total_bytes)
    .execute(pool)
    .await
    .context("Failed to update download progress")?;

    Ok(())
}

/// Pause a download.
#[tracing::instrument(name = "Pause download", skip(pool))]
pub async fn pause_download(download_id: Uuid, pool: &PgPool) -> Result<Download> {
    update_download_status(download_id, DownloadStatus::Paused, None, None, pool).await
}

/// Resume a download (sets status back to Pending for re-processing).
#[tracing::instrument(name = "Resume download", skip(pool))]
pub async fn resume_download(download_id: Uuid, pool: &PgPool) -> Result<Download> {
    update_download_status(download_id, DownloadStatus::InProgress, None, None, pool).await
}

/// Retry a failed download.
#[tracing::instrument(name = "Retry download", skip(pool))]
pub async fn retry_download(download_id: Uuid, pool: &PgPool) -> Result<Download> {
    let query = format!(
        r#"
        UPDATE downloads
        SET status = $2, error_message = NULL, updated_at = NOW()
        WHERE id = $1
        RETURNING {DOWNLOAD_COLUMNS}
        "#
    );

    let row = sqlx::query_as::<_, DownloadRow>(&query)
        .bind(download_id)
        .bind(DownloadStatus::Pending)
        .fetch_one(pool)
        .await
        .context("Failed to retry download")?;

    Ok(Download::from(row))
}

/// Cancel a download.
#[tracing::instrument(name = "Cancel download", skip(pool))]
pub async fn cancel_download(download_id: Uuid, pool: &PgPool) -> Result<Download> {
    update_download_status(download_id, DownloadStatus::Cancelled, None, None, pool).await
}

/// Get all downloads for a user.
#[tracing::instrument(name = "Get all downloads", skip(pool))]
pub async fn get_all_downloads(pool: &PgPool, user_id: &UserId) -> Result<Vec<Download>> {
    let query = format!(
        "SELECT {DOWNLOAD_COLUMNS} FROM downloads WHERE user_id = $1 ORDER BY created_at DESC"
    );

    let rows = sqlx::query_as::<_, DownloadRow>(&query)
        .bind(user_id.0)
        .fetch_all(pool)
        .await
        .context("Failed to fetch all downloads")?;

    Ok(rows.into_iter().map(Download::from).collect())
}

/// Get downloads with pagination, filtering, and sorting.
#[tracing::instrument(
    name = "Get paginated downloads",
    skip(pool, pagination, filter, sort)
)]
pub async fn get_downloads_paginated(
    pool: &PgPool,
    user_id: &UserId,
    pagination: &PaginationParams,
    filter: &DownloadFilter,
    sort: &SortParams,
) -> Result<PaginatedResponse<Download>> {
    // Build the base WHERE clause
    let mut conditions = vec!["user_id = $1".to_string()];
    let mut param_count = 1;

    if filter.status.is_some() {
        param_count += 1;
        conditions.push(format!("status = ${param_count}"));
    }

    if filter.url_contains.is_some() {
        param_count += 1;
        conditions.push(format!("url ILIKE ${param_count}"));
    }

    if filter.created_after.is_some() {
        param_count += 1;
        conditions.push(format!("created_at >= ${param_count}"));
    }

    if filter.created_before.is_some() {
        param_count += 1;
        conditions.push(format!("created_at <= ${param_count}"));
    }

    if filter.completed_after.is_some() {
        param_count += 1;
        conditions.push(format!("completed_at >= ${param_count}"));
    }

    if filter.completed_before.is_some() {
        param_count += 1;
        conditions.push(format!("completed_at <= ${param_count}"));
    }

    let where_clause = conditions.join(" AND ");
    let order_by = sort.to_order_by_or_default(
        DOWNLOAD_SORT_FIELDS,
        DOWNLOAD_DEFAULT_SORT,
        DOWNLOAD_DEFAULT_ORDER,
    );

    let count_query = format!("SELECT COUNT(*) as count FROM downloads WHERE {where_clause}");
    let data_query = format!(
        "SELECT {DOWNLOAD_COLUMNS} FROM downloads WHERE {where_clause} ORDER BY {order_by} LIMIT ${} OFFSET ${}",
        param_count + 1,
        param_count + 2
    );

    let total = execute_count_query(pool, &count_query, user_id, filter).await?;
    let downloads = execute_data_query(pool, &data_query, user_id, filter, pagination).await?;

    Ok(PaginatedResponse::new(downloads, pagination, total))
}

/// Execute count query with dynamic filter bindings.
async fn execute_count_query(
    pool: &PgPool,
    query: &str,
    user_id: &UserId,
    filter: &DownloadFilter,
) -> Result<i64> {
    use sqlx::Row;

    let mut query_builder = sqlx::query(query).bind(user_id.0);

    if let Some(status) = filter.status {
        query_builder = query_builder.bind(status);
    }

    if let Some(ref url_contains) = filter.url_contains {
        query_builder = query_builder.bind(format!("%{url_contains}%"));
    }

    if let Some(created_after) = filter.created_after {
        query_builder = query_builder.bind(created_after);
    }

    if let Some(created_before) = filter.created_before {
        query_builder = query_builder.bind(created_before);
    }

    if let Some(completed_after) = filter.completed_after {
        query_builder = query_builder.bind(completed_after);
    }

    if let Some(completed_before) = filter.completed_before {
        query_builder = query_builder.bind(completed_before);
    }

    let row = query_builder
        .fetch_one(pool)
        .await
        .context("Failed to count downloads")?;

    Ok(row.get::<i64, _>("count"))
}

/// Execute data query with dynamic filter bindings.
async fn execute_data_query(
    pool: &PgPool,
    query: &str,
    user_id: &UserId,
    filter: &DownloadFilter,
    pagination: &PaginationParams,
) -> Result<Vec<Download>> {
    let mut query_builder = sqlx::query_as::<_, DownloadRow>(query).bind(user_id.0);

    if let Some(status) = filter.status {
        query_builder = query_builder.bind(status);
    }

    if let Some(ref url_contains) = filter.url_contains {
        query_builder = query_builder.bind(format!("%{url_contains}%"));
    }

    if let Some(created_after) = filter.created_after {
        query_builder = query_builder.bind(created_after);
    }

    if let Some(created_before) = filter.created_before {
        query_builder = query_builder.bind(created_before);
    }

    if let Some(completed_after) = filter.completed_after {
        query_builder = query_builder.bind(completed_after);
    }

    if let Some(completed_before) = filter.completed_before {
        query_builder = query_builder.bind(completed_before);
    }

    query_builder = query_builder.bind(pagination.limit()).bind(pagination.offset());

    let rows = query_builder
        .fetch_all(pool)
        .await
        .context("Failed to fetch paginated downloads")?;

    Ok(rows.into_iter().map(Download::from).collect())
}
