use crate::{
    middlewares::UserId,
    models::{
        Download, DownloadFilter, DownloadStatus, PaginatedResponse, PaginationParams, SortParams,
        sorting::{DOWNLOAD_DEFAULT_ORDER, DOWNLOAD_DEFAULT_SORT, DOWNLOAD_SORT_FIELDS},
    },
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
        },
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

/// Get downloads with pagination, filtering, and sorting.
#[tracing::instrument(
    name = "Get paginated downloads from database",
    skip(pool, pagination, filter, sort)
)]
pub async fn get_downloads_paginated(
    pool: &PgPool,
    user_id: &UserId,
    pagination: &PaginationParams,
    filter: &DownloadFilter,
    sort: &SortParams,
) -> Result<PaginatedResponse<Download>, anyhow::Error> {
    // Build the base WHERE clause
    let mut conditions = vec!["user_id = $1".to_string()];
    let mut param_count = 1;

    // Add filter conditions
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

    // Build ORDER BY clause
    let order_by =
        sort.to_order_by_or_default(DOWNLOAD_SORT_FIELDS, DOWNLOAD_DEFAULT_SORT, DOWNLOAD_DEFAULT_ORDER);

    // First, get the total count
    let count_query = format!("SELECT COUNT(*) as count FROM downloads WHERE {where_clause}");

    // Build the data query
    let data_query = format!(
        "SELECT id, url, status, file_path, user_id, created_at, updated_at, completed_at \
         FROM downloads WHERE {where_clause} ORDER BY {order_by} LIMIT ${}  OFFSET ${}",
        param_count + 1,
        param_count + 2
    );

    // Execute count query with dynamic bindings
    let total: i64 = execute_count_query(pool, &count_query, user_id, filter).await?;

    // Execute data query with dynamic bindings
    let downloads = execute_data_query(pool, &data_query, user_id, filter, pagination).await?;

    Ok(PaginatedResponse::new(downloads, pagination, total))
}

/// Execute the count query with dynamic filter bindings.
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

/// Execute the data query with dynamic filter bindings.
async fn execute_data_query(
    pool: &PgPool,
    query: &str,
    user_id: &UserId,
    filter: &DownloadFilter,
    pagination: &PaginationParams,
) -> Result<Vec<Download>> {
    let mut query_builder = sqlx::query_as::<_, Download>(query).bind(user_id.0);

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

    // Add pagination params
    query_builder = query_builder.bind(pagination.limit()).bind(pagination.offset());

    let downloads = query_builder
        .fetch_all(pool)
        .await
        .context("Failed to fetch paginated downloads")?;

    Ok(downloads)
}
