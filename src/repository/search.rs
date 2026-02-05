//! Search repository for full-text search operations.

use crate::{
    middlewares::UserId,
    models::{Download, DownloadRow, Folder, SearchQuery, SearchResponse, SearchResult, SearchResultType},
};
use anyhow::{Context, Result};
use sqlx::PgPool;
use std::time::Instant;

/// Perform a full-text search across downloads and folders.
#[tracing::instrument(name = "Search", skip(pool))]
pub async fn search(
    query: &SearchQuery,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<SearchResponse> {
    let start = Instant::now();

    // Escape and prepare the search query
    let search_term = prepare_search_query(&query.q);

    if search_term.is_empty() {
        return Ok(SearchResponse::empty());
    }

    let mut results = Vec::new();
    let mut total: i64 = 0;

    // Search downloads
    let (download_results, download_count) = search_downloads(&search_term, query, user_id, pool).await?;
    results.extend(download_results);
    total += download_count;

    // Search folders
    let (folder_results, folder_count) = search_folders(&search_term, query, user_id, pool).await?;
    results.extend(folder_results);
    total += folder_count;

    // Sort by score (descending)
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Apply pagination
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let offset = query.offset() as usize;
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let limit = query.limit() as usize;
    let paginated_results: Vec<SearchResult> = results
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    #[allow(clippy::cast_possible_truncation)]
    let took_ms = start.elapsed().as_millis() as i64;

    Ok(SearchResponse {
        results: paginated_results,
        total,
        took_ms,
        suggestions: Vec::new(), // Suggestions can be implemented later
    })
}

/// Prepare search query for PostgreSQL full-text search.
/// Strips all tsquery special characters to prevent injection.
fn prepare_search_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .filter_map(|word| {
            // Keep only alphanumeric characters, hyphens, and underscores
            let sanitized: String = word
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if sanitized.is_empty() {
                None
            } else {
                Some(format!("{sanitized}:*"))
            }
        })
        .collect::<Vec<_>>()
        .join(" & ")
}

/// Escape ILIKE special characters to prevent wildcard injection.
fn escape_ilike_search(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Search downloads.
#[allow(clippy::too_many_lines)]
async fn search_downloads(
    search_term: &str,
    query: &SearchQuery,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<(Vec<SearchResult>, i64)> {
    let mut conditions = vec![
        "user_id = $1".to_string(),
        "search_vector @@ to_tsquery('english', $2)".to_string(),
    ];
    let mut param_count = 2;

    // Apply additional filters
    if query.folder_id.is_some() {
        param_count += 1;
        conditions.push(format!("folder_id = ${param_count}"));
    }

    if let Some(ref statuses) = query.status
        && !statuses.is_empty() {
        param_count += 1;
        conditions.push(format!("status = ANY(${param_count})"));
    }

    if query.content_type.is_some() {
        param_count += 1;
        conditions.push(format!("content_type ILIKE ${param_count}"));
    }

    if query.min_size.is_some() {
        param_count += 1;
        conditions.push(format!("total_bytes >= ${param_count}"));
    }

    if query.max_size.is_some() {
        param_count += 1;
        conditions.push(format!("total_bytes <= ${param_count}"));
    }

    if query.created_after.is_some() {
        param_count += 1;
        conditions.push(format!("created_at >= ${param_count}"));
    }

    if query.created_before.is_some() {
        param_count += 1;
        conditions.push(format!("created_at <= ${param_count}"));
    }

    let where_clause = conditions.join(" AND ");

    // Count query
    let count_query = format!(
        "SELECT COUNT(*) FROM downloads WHERE {where_clause}"
    );

    // Fetch enough rows to cover the requested page across merged results
    let fetch_limit = query.offset() + query.limit();

    // Data query with relevance score
    let data_query = format!(
        r"
        SELECT
            id, url, status, file_path, user_id, bytes_downloaded, total_bytes,
            content_type, filename, error_message, retry_count, max_retries,
            priority, started_at, metadata, created_at, updated_at, completed_at,
            ts_rank(search_vector, to_tsquery('english', $2)) as rank
        FROM downloads
        WHERE {where_clause}
        ORDER BY rank DESC
        LIMIT {fetch_limit}
        "
    );

    // Build count query
    let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query)
        .bind(user_id.0)
        .bind(search_term);

    if let Some(folder_id) = query.folder_id {
        count_builder = count_builder.bind(folder_id);
    }
    if let Some(ref statuses) = query.status
        && !statuses.is_empty() {
            let status_strs: Vec<String> = statuses.iter().map(std::string::ToString::to_string).collect();
            count_builder = count_builder.bind(status_strs);
        }
    if let Some(ref content_type) = query.content_type {
        count_builder = count_builder.bind(format!("%{}%", escape_ilike_search(content_type)));
    }
    if let Some(min_size) = query.min_size {
        count_builder = count_builder.bind(min_size);
    }
    if let Some(max_size) = query.max_size {
        count_builder = count_builder.bind(max_size);
    }
    if let Some(created_after) = query.created_after {
        count_builder = count_builder.bind(created_after);
    }
    if let Some(created_before) = query.created_before {
        count_builder = count_builder.bind(created_before);
    }

    let count = count_builder
        .fetch_one(pool)
        .await
        .context("Failed to count search results")?;

    // Build data query
    let mut data_builder = sqlx::query_as::<_, DownloadSearchRow>(&data_query)
        .bind(user_id.0)
        .bind(search_term);

    if let Some(folder_id) = query.folder_id {
        data_builder = data_builder.bind(folder_id);
    }
    if let Some(ref statuses) = query.status
        && !statuses.is_empty() {
            let status_strs: Vec<String> = statuses.iter().map(std::string::ToString::to_string).collect();
            data_builder = data_builder.bind(status_strs);
        }
    if let Some(ref content_type) = query.content_type {
        data_builder = data_builder.bind(format!("%{}%", escape_ilike_search(content_type)));
    }
    if let Some(min_size) = query.min_size {
        data_builder = data_builder.bind(min_size);
    }
    if let Some(max_size) = query.max_size {
        data_builder = data_builder.bind(max_size);
    }
    if let Some(created_after) = query.created_after {
        data_builder = data_builder.bind(created_after);
    }
    if let Some(created_before) = query.created_before {
        data_builder = data_builder.bind(created_before);
    }

    let rows = data_builder
        .fetch_all(pool)
        .await
        .context("Failed to search downloads")?;

    let results: Vec<SearchResult> = rows
        .into_iter()
        .map(|row| {
            let download = Download::from(DownloadRow {
                id: row.id,
                url: row.url.clone(),
                status: row.status,
                file_path: row.file_path.clone(),
                user_id: row.user_id,
                bytes_downloaded: row.bytes_downloaded,
                total_bytes: row.total_bytes,
                content_type: row.content_type.clone(),
                filename: row.filename.clone(),
                error_message: row.error_message.clone(),
                retry_count: row.retry_count,
                max_retries: row.max_retries,
                priority: row.priority,
                started_at: row.started_at,
                metadata: row.metadata.clone(),
                created_at: row.created_at,
                updated_at: row.updated_at,
                completed_at: row.completed_at,
            });

            SearchResult {
                id: row.id,
                result_type: SearchResultType::Download,
                title: row.filename.unwrap_or_else(|| row.url.clone()),
                snippet: None,
                score: row.rank,
                data: serde_json::to_value(&download).unwrap_or_default(),
            }
        })
        .collect();

    Ok((results, count))
}

/// Search folders.
async fn search_folders(
    search_term: &str,
    query: &SearchQuery,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<(Vec<SearchResult>, i64)> {
    // If folder_id is specified, we're searching within a folder, so don't search folder names
    if query.folder_id.is_some() {
        return Ok((Vec::new(), 0));
    }

    let count_query = r"
        SELECT COUNT(*)
        FROM folders
        WHERE user_id = $1 AND search_vector @@ to_tsquery('english', $2)
    ";

    let fetch_limit = query.offset() + query.limit();
    let data_query = format!(
        r"
        SELECT
            id, user_id, parent_id, name, path, created_at, updated_at,
            ts_rank(search_vector, to_tsquery('english', $2)) as rank
        FROM folders
        WHERE user_id = $1 AND search_vector @@ to_tsquery('english', $2)
        ORDER BY rank DESC
        LIMIT {fetch_limit}
        "
    );

    let count = sqlx::query_scalar::<_, i64>(count_query)
        .bind(user_id.0)
        .bind(search_term)
        .fetch_one(pool)
        .await
        .context("Failed to count folder search results")?;

    let rows = sqlx::query_as::<_, FolderSearchRow>(&data_query)
        .bind(user_id.0)
        .bind(search_term)
        .fetch_all(pool)
        .await
        .context("Failed to search folders")?;

    let results: Vec<SearchResult> = rows
        .into_iter()
        .map(|row| {
            let folder = Folder {
                id: row.id,
                user_id: row.user_id,
                parent_id: row.parent_id,
                name: row.name.clone(),
                path: row.path.clone(),
                created_at: row.created_at,
                updated_at: row.updated_at,
            };

            SearchResult {
                id: row.id,
                result_type: SearchResultType::Folder,
                title: row.name,
                snippet: Some(row.path),
                score: row.rank,
                data: serde_json::to_value(&folder).unwrap_or_default(),
            }
        })
        .collect();

    Ok((results, count))
}

/// Download row with search rank.
#[derive(Debug, sqlx::FromRow)]
struct DownloadSearchRow {
    pub id: uuid::Uuid,
    pub url: String,
    pub status: crate::models::DownloadStatus,
    pub file_path: Option<String>,
    pub user_id: uuid::Uuid,
    pub bytes_downloaded: Option<i64>,
    pub total_bytes: Option<i64>,
    pub content_type: Option<String>,
    pub filename: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: Option<i32>,
    pub max_retries: Option<i32>,
    pub priority: Option<i32>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub rank: f32,
}

/// Folder row with search rank.
#[derive(Debug, sqlx::FromRow)]
struct FolderSearchRow {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub parent_id: Option<uuid::Uuid>,
    pub name: String,
    pub path: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub rank: f32,
}
