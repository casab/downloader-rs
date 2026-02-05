//! Tag repository for tag management operations.

use crate::{
    middlewares::UserId,
    models::{Tag, TagRow, TagWithCount},
};
use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

/// All columns for tag queries.
const TAG_COLUMNS: &str = r"
    id, user_id, name, color, created_at
";

/// Get a tag by ID.
#[tracing::instrument(name = "Get tag by ID", skip(pool))]
pub async fn get_tag_by_id(tag_id: Uuid, pool: &PgPool) -> Result<Option<Tag>> {
    let query = format!("SELECT {TAG_COLUMNS} FROM tags WHERE id = $1");

    let row = sqlx::query_as::<_, TagRow>(&query)
        .bind(tag_id)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch tag by ID")?;

    Ok(row.map(Tag::from))
}

/// Get a tag by ID for a specific user (ownership check).
#[tracing::instrument(name = "Get user tag by ID", skip(pool))]
pub async fn get_user_tag_by_id(
    tag_id: Uuid,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Option<Tag>> {
    let query = format!("SELECT {TAG_COLUMNS} FROM tags WHERE id = $1 AND user_id = $2");

    let row = sqlx::query_as::<_, TagRow>(&query)
        .bind(tag_id)
        .bind(user_id.0)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch user tag by ID")?;

    Ok(row.map(Tag::from))
}

/// Get a tag by name for a user.
#[tracing::instrument(name = "Get tag by name", skip(pool))]
pub async fn get_tag_by_name(name: &str, user_id: &UserId, pool: &PgPool) -> Result<Option<Tag>> {
    let query = format!("SELECT {TAG_COLUMNS} FROM tags WHERE name = $1 AND user_id = $2");

    let row = sqlx::query_as::<_, TagRow>(&query)
        .bind(name)
        .bind(user_id.0)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch tag by name")?;

    Ok(row.map(Tag::from))
}

/// Create a new tag.
#[tracing::instrument(name = "Create tag", skip(pool))]
pub async fn create_tag(
    name: &str,
    color: Option<&str>,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Tag> {
    let query = format!(
        r"
        INSERT INTO tags (id, user_id, name, color)
        VALUES ($1, $2, $3, $4)
        RETURNING {TAG_COLUMNS}
        "
    );

    let row = sqlx::query_as::<_, TagRow>(&query)
        .bind(Uuid::new_v4())
        .bind(user_id.0)
        .bind(name)
        .bind(color)
        .fetch_one(pool)
        .await
        .context("Failed to create tag")?;

    Ok(Tag::from(row))
}

/// Update a tag.
#[tracing::instrument(name = "Update tag", skip(pool))]
pub async fn update_tag(
    tag_id: Uuid,
    name: Option<&str>,
    color: Option<&str>,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Tag> {
    let query = format!(
        r"
        UPDATE tags
        SET name = COALESCE($3, name), color = COALESCE($4, color)
        WHERE id = $1 AND user_id = $2
        RETURNING {TAG_COLUMNS}
        "
    );

    let row = sqlx::query_as::<_, TagRow>(&query)
        .bind(tag_id)
        .bind(user_id.0)
        .bind(name)
        .bind(color)
        .fetch_one(pool)
        .await
        .context("Failed to update tag")?;

    Ok(Tag::from(row))
}

/// Delete a tag.
#[tracing::instrument(name = "Delete tag", skip(pool))]
pub async fn delete_tag(tag_id: Uuid, user_id: &UserId, pool: &PgPool) -> Result<()> {
    let result = sqlx::query("DELETE FROM tags WHERE id = $1 AND user_id = $2")
        .bind(tag_id)
        .bind(user_id.0)
        .execute(pool)
        .await
        .context("Failed to delete tag")?;

    if result.rows_affected() == 0 {
        return Err(anyhow::anyhow!("Tag not found"));
    }

    Ok(())
}

/// Get all tags for a user with usage counts.
#[tracing::instrument(name = "Get user tags", skip(pool))]
pub async fn get_user_tags(user_id: &UserId, pool: &PgPool) -> Result<Vec<TagWithCount>> {
    let query = r"
        SELECT
            t.id, t.user_id, t.name, t.color, t.created_at,
            COUNT(dt.download_id) as download_count
        FROM tags t
        LEFT JOIN download_tags dt ON dt.tag_id = t.id
        WHERE t.user_id = $1
        GROUP BY t.id
        ORDER BY t.name ASC
    ";

    let rows = sqlx::query_as::<_, TagWithCountRow>(query)
        .bind(user_id.0)
        .fetch_all(pool)
        .await
        .context("Failed to fetch user tags")?;

    Ok(rows.into_iter().map(TagWithCount::from).collect())
}

/// Add tags to a download.
#[tracing::instrument(name = "Add tags to download", skip(pool))]
pub async fn add_tags_to_download(
    download_id: Uuid,
    tag_ids: &[Uuid],
    user_id: &UserId,
    pool: &PgPool,
) -> Result<i64> {
    // Verify download belongs to user
    let download_check = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM downloads WHERE id = $1 AND user_id = $2",
    )
    .bind(download_id)
    .bind(user_id.0)
    .fetch_one(pool)
    .await?;

    if download_check == 0 {
        return Err(anyhow::anyhow!("Download not found"));
    }

    // Verify tags belong to user
    let tags_check = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tags WHERE id = ANY($1) AND user_id = $2",
    )
    .bind(tag_ids)
    .bind(user_id.0)
    .fetch_one(pool)
    .await?;

    #[allow(clippy::cast_possible_wrap)]
    let expected_tag_count = tag_ids.len() as i64;
    if tags_check != expected_tag_count {
        return Err(anyhow::anyhow!("One or more tags not found"));
    }

    // Insert tags (ignore conflicts)
    let mut added = 0i64;
    for tag_id in tag_ids {
        let result = sqlx::query(
            r"
            INSERT INTO download_tags (download_id, tag_id)
            VALUES ($1, $2)
            ON CONFLICT (download_id, tag_id) DO NOTHING
            ",
        )
        .bind(download_id)
        .bind(tag_id)
        .execute(pool)
        .await
        .context("Failed to add tag to download")?;

        #[allow(clippy::cast_possible_wrap)]
        let affected = result.rows_affected() as i64;
        added += affected;
    }

    Ok(added)
}

/// Remove a tag from a download.
#[tracing::instrument(name = "Remove tag from download", skip(pool))]
pub async fn remove_tag_from_download(
    download_id: Uuid,
    tag_id: Uuid,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<()> {
    // Verify download belongs to user
    let download_check = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM downloads WHERE id = $1 AND user_id = $2",
    )
    .bind(download_id)
    .bind(user_id.0)
    .fetch_one(pool)
    .await?;

    if download_check == 0 {
        return Err(anyhow::anyhow!("Download not found"));
    }

    let result = sqlx::query("DELETE FROM download_tags WHERE download_id = $1 AND tag_id = $2")
        .bind(download_id)
        .bind(tag_id)
        .execute(pool)
        .await
        .context("Failed to remove tag from download")?;

    if result.rows_affected() == 0 {
        return Err(anyhow::anyhow!("Tag not found on download"));
    }

    Ok(())
}

/// Get tags for a download.
#[tracing::instrument(name = "Get download tags", skip(pool))]
pub async fn get_download_tags(download_id: Uuid, pool: &PgPool) -> Result<Vec<Tag>> {
    let query = format!(
        r"
        SELECT {TAG_COLUMNS}
        FROM tags t
        JOIN download_tags dt ON dt.tag_id = t.id
        WHERE dt.download_id = $1
        ORDER BY t.name ASC
        "
    );

    let rows = sqlx::query_as::<_, TagRow>(&query)
        .bind(download_id)
        .fetch_all(pool)
        .await
        .context("Failed to fetch download tags")?;

    Ok(rows.into_iter().map(Tag::from).collect())
}

/// Bulk add tags to multiple downloads.
#[tracing::instrument(name = "Bulk add tags", skip(pool))]
pub async fn bulk_add_tags(
    download_ids: &[Uuid],
    tag_ids: &[Uuid],
    user_id: &UserId,
    pool: &PgPool,
) -> Result<i64> {
    // Verify downloads belong to user
    let downloads_check = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM downloads WHERE id = ANY($1) AND user_id = $2",
    )
    .bind(download_ids)
    .bind(user_id.0)
    .fetch_one(pool)
    .await?;

    #[allow(clippy::cast_possible_wrap)]
    let expected_download_count = download_ids.len() as i64;
    if downloads_check != expected_download_count {
        return Err(anyhow::anyhow!("One or more downloads not found"));
    }

    // Verify tags belong to user
    let tags_check = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tags WHERE id = ANY($1) AND user_id = $2",
    )
    .bind(tag_ids)
    .bind(user_id.0)
    .fetch_one(pool)
    .await?;

    #[allow(clippy::cast_possible_wrap)]
    let expected_tag_count = tag_ids.len() as i64;
    if tags_check != expected_tag_count {
        return Err(anyhow::anyhow!("One or more tags not found"));
    }

    // Insert all combinations
    let mut added = 0i64;
    for download_id in download_ids {
        for tag_id in tag_ids {
            let result = sqlx::query(
                r"
                INSERT INTO download_tags (download_id, tag_id)
                VALUES ($1, $2)
                ON CONFLICT (download_id, tag_id) DO NOTHING
                ",
            )
            .bind(download_id)
            .bind(tag_id)
            .execute(pool)
            .await
            .context("Failed to add tag to download")?;

            #[allow(clippy::cast_possible_wrap)]
            let affected = result.rows_affected() as i64;
            added += affected;
        }
    }

    Ok(added)
}

/// Bulk remove tags from multiple downloads.
#[tracing::instrument(name = "Bulk remove tags", skip(pool))]
pub async fn bulk_remove_tags(
    download_ids: &[Uuid],
    tag_ids: &[Uuid],
    user_id: &UserId,
    pool: &PgPool,
) -> Result<i64> {
    // Verify downloads belong to user
    let downloads_check = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM downloads WHERE id = ANY($1) AND user_id = $2",
    )
    .bind(download_ids)
    .bind(user_id.0)
    .fetch_one(pool)
    .await?;

    #[allow(clippy::cast_possible_wrap)]
    let expected_download_count = download_ids.len() as i64;
    if downloads_check != expected_download_count {
        return Err(anyhow::anyhow!("One or more downloads not found"));
    }

    let result = sqlx::query(
        r"
        DELETE FROM download_tags
        WHERE download_id = ANY($1) AND tag_id = ANY($2)
        ",
    )
    .bind(download_ids)
    .bind(tag_ids)
    .execute(pool)
    .await
    .context("Failed to remove tags from downloads")?;

    #[allow(clippy::cast_possible_wrap)]
    let count = result.rows_affected() as i64;
    Ok(count)
}

/// Helper row type for tag with count.
#[derive(Debug, sqlx::FromRow)]
struct TagWithCountRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub download_count: i64,
}

impl From<TagWithCountRow> for TagWithCount {
    fn from(row: TagWithCountRow) -> Self {
        Self {
            tag: Tag {
                id: row.id,
                user_id: row.user_id,
                name: row.name,
                color: row.color,
                created_at: row.created_at,
            },
            download_count: row.download_count,
        }
    }
}
