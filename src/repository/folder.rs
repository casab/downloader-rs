//! Folder repository for folder management operations.

use crate::{
    middlewares::UserId,
    models::{Folder, FolderRow, FolderWithCounts},
};
use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

/// All columns for folder queries.
const FOLDER_COLUMNS: &str = r"
    id, user_id, parent_id, name, path, created_at, updated_at
";

/// Get a folder by ID.
#[tracing::instrument(name = "Get folder by ID", skip(pool))]
pub async fn get_folder_by_id(folder_id: Uuid, pool: &PgPool) -> Result<Option<Folder>> {
    let query = format!("SELECT {FOLDER_COLUMNS} FROM folders WHERE id = $1");

    let row = sqlx::query_as::<_, FolderRow>(&query)
        .bind(folder_id)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch folder by ID")?;

    Ok(row.map(Folder::from))
}

/// Get a folder by ID for a specific user (ownership check).
#[tracing::instrument(name = "Get user folder by ID", skip(pool))]
pub async fn get_user_folder_by_id(
    folder_id: Uuid,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Option<Folder>> {
    let query = format!(
        "SELECT {FOLDER_COLUMNS} FROM folders WHERE id = $1 AND user_id = $2"
    );

    let row = sqlx::query_as::<_, FolderRow>(&query)
        .bind(folder_id)
        .bind(user_id.0)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch user folder by ID")?;

    Ok(row.map(Folder::from))
}

/// Get a folder by path for a user.
#[tracing::instrument(name = "Get folder by path", skip(pool))]
pub async fn get_folder_by_path(
    path: &str,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Option<Folder>> {
    let query = format!(
        "SELECT {FOLDER_COLUMNS} FROM folders WHERE path = $1 AND user_id = $2"
    );

    let row = sqlx::query_as::<_, FolderRow>(&query)
        .bind(path)
        .bind(user_id.0)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch folder by path")?;

    Ok(row.map(Folder::from))
}

/// Create a new folder.
#[tracing::instrument(name = "Create folder", skip(pool))]
pub async fn create_folder(
    name: &str,
    parent_id: Option<Uuid>,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Folder> {
    // Build path based on parent
    let path = if let Some(pid) = parent_id {
        let parent = get_user_folder_by_id(pid, user_id, pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Parent folder not found"))?;
        format!("{}/{}", parent.path, name)
    } else {
        format!("/{name}")
    };

    let query = format!(
        r"
        INSERT INTO folders (id, user_id, parent_id, name, path)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING {FOLDER_COLUMNS}
        "
    );

    let row = sqlx::query_as::<_, FolderRow>(&query)
        .bind(Uuid::new_v4())
        .bind(user_id.0)
        .bind(parent_id)
        .bind(name)
        .bind(&path)
        .fetch_one(pool)
        .await
        .context("Failed to create folder")?;

    Ok(Folder::from(row))
}

/// Update a folder's name.
#[tracing::instrument(name = "Update folder name", skip(pool))]
pub async fn update_folder_name(
    folder_id: Uuid,
    name: &str,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Folder> {
    // Get the existing folder
    let folder = get_user_folder_by_id(folder_id, user_id, pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Folder not found"))?;

    // Calculate new path
    let new_path = if let Some(parent_path) = folder.parent_path() {
        format!("{parent_path}/{name}")
    } else {
        format!("/{name}")
    };

    // Update this folder and all descendants
    let old_path = &folder.path;

    // Update the folder itself
    let query = format!(
        r"
        UPDATE folders
        SET name = $3, path = $4, updated_at = NOW()
        WHERE id = $1 AND user_id = $2
        RETURNING {FOLDER_COLUMNS}
        "
    );

    let row = sqlx::query_as::<_, FolderRow>(&query)
        .bind(folder_id)
        .bind(user_id.0)
        .bind(name)
        .bind(&new_path)
        .fetch_one(pool)
        .await
        .context("Failed to update folder name")?;

    // Update all descendant paths
    sqlx::query(
        r"
        UPDATE folders
        SET path = $3 || substring(path from length($1) + 1), updated_at = NOW()
        WHERE user_id = $2 AND path LIKE $1 || '/%'
        ",
    )
    .bind(old_path)
    .bind(user_id.0)
    .bind(&new_path)
    .execute(pool)
    .await
    .context("Failed to update descendant folder paths")?;

    Ok(Folder::from(row))
}

/// Move a folder to a new parent.
#[tracing::instrument(name = "Move folder", skip(pool))]
pub async fn move_folder(
    folder_id: Uuid,
    new_parent_id: Option<Uuid>,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Folder> {
    // Get the existing folder
    let folder = get_user_folder_by_id(folder_id, user_id, pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Folder not found"))?;

    // Check for circular reference
    if let Some(pid) = new_parent_id {
        if pid == folder_id {
            return Err(anyhow::anyhow!("Cannot move folder into itself"));
        }
        let new_parent = get_user_folder_by_id(pid, user_id, pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Target parent folder not found"))?;

        // Check if new parent is a descendant of this folder
        if folder.is_ancestor_of(&new_parent) {
            return Err(anyhow::anyhow!("Cannot move folder into its own descendant"));
        }
    }

    // Calculate new path
    let new_path = if let Some(pid) = new_parent_id {
        let parent = get_user_folder_by_id(pid, user_id, pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Target parent folder not found"))?;
        format!("{}/{}", parent.path, folder.name)
    } else {
        format!("/{}", folder.name)
    };

    let old_path = &folder.path;

    // Update the folder itself
    let query = format!(
        r"
        UPDATE folders
        SET parent_id = $3, path = $4, updated_at = NOW()
        WHERE id = $1 AND user_id = $2
        RETURNING {FOLDER_COLUMNS}
        "
    );

    let row = sqlx::query_as::<_, FolderRow>(&query)
        .bind(folder_id)
        .bind(user_id.0)
        .bind(new_parent_id)
        .bind(&new_path)
        .fetch_one(pool)
        .await
        .context("Failed to move folder")?;

    // Update all descendant paths
    sqlx::query(
        r"
        UPDATE folders
        SET path = $3 || substring(path from length($1) + 1), updated_at = NOW()
        WHERE user_id = $2 AND path LIKE $1 || '/%'
        ",
    )
    .bind(old_path)
    .bind(user_id.0)
    .bind(&new_path)
    .execute(pool)
    .await
    .context("Failed to update descendant folder paths")?;

    Ok(Folder::from(row))
}

/// Delete a folder and all its contents.
#[tracing::instrument(name = "Delete folder", skip(pool))]
pub async fn delete_folder(
    folder_id: Uuid,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<()> {
    // First, set all downloads in this folder to no folder
    sqlx::query(
        r"
        UPDATE downloads
        SET folder_id = NULL
        WHERE folder_id = $1 AND user_id = $2
        ",
    )
    .bind(folder_id)
    .bind(user_id.0)
    .execute(pool)
    .await
    .context("Failed to unlink downloads from folder")?;

    // Delete the folder (CASCADE will delete children)
    let result = sqlx::query(
        "DELETE FROM folders WHERE id = $1 AND user_id = $2",
    )
    .bind(folder_id)
    .bind(user_id.0)
    .execute(pool)
    .await
    .context("Failed to delete folder")?;

    if result.rows_affected() == 0 {
        return Err(anyhow::anyhow!("Folder not found"));
    }

    Ok(())
}

/// Get all root folders for a user.
#[tracing::instrument(name = "Get root folders", skip(pool))]
pub async fn get_root_folders(user_id: &UserId, pool: &PgPool) -> Result<Vec<FolderWithCounts>> {
    let query = r"
        SELECT
            f.id, f.user_id, f.parent_id, f.name, f.path, f.created_at, f.updated_at,
            (SELECT COUNT(*) FROM folders WHERE parent_id = f.id) as subfolder_count,
            (SELECT COUNT(*) FROM downloads WHERE folder_id = f.id) as download_count
        FROM folders f
        WHERE f.user_id = $1 AND f.parent_id IS NULL
        ORDER BY f.name ASC
    ";

    let rows = sqlx::query_as::<_, FolderWithCountsRow>(query)
        .bind(user_id.0)
        .fetch_all(pool)
        .await
        .context("Failed to fetch root folders")?;

    Ok(rows.into_iter().map(FolderWithCounts::from).collect())
}

/// Get folder contents (subfolders and download count).
#[tracing::instrument(name = "Get folder contents", skip(pool))]
pub async fn get_folder_with_contents(
    folder_id: Uuid,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Option<FolderWithCounts>> {
    let query = r"
        SELECT
            f.id, f.user_id, f.parent_id, f.name, f.path, f.created_at, f.updated_at,
            (SELECT COUNT(*) FROM folders WHERE parent_id = f.id) as subfolder_count,
            (SELECT COUNT(*) FROM downloads WHERE folder_id = f.id) as download_count
        FROM folders f
        WHERE f.id = $1 AND f.user_id = $2
    ";

    let row = sqlx::query_as::<_, FolderWithCountsRow>(query)
        .bind(folder_id)
        .bind(user_id.0)
        .fetch_optional(pool)
        .await
        .context("Failed to fetch folder contents")?;

    Ok(row.map(FolderWithCounts::from))
}

/// Get subfolders of a folder.
#[tracing::instrument(name = "Get subfolders", skip(pool))]
pub async fn get_subfolders(
    folder_id: Uuid,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<Vec<FolderWithCounts>> {
    let query = r"
        SELECT
            f.id, f.user_id, f.parent_id, f.name, f.path, f.created_at, f.updated_at,
            (SELECT COUNT(*) FROM folders WHERE parent_id = f.id) as subfolder_count,
            (SELECT COUNT(*) FROM downloads WHERE folder_id = f.id) as download_count
        FROM folders f
        WHERE f.parent_id = $1 AND f.user_id = $2
        ORDER BY f.name ASC
    ";

    let rows = sqlx::query_as::<_, FolderWithCountsRow>(query)
        .bind(folder_id)
        .bind(user_id.0)
        .fetch_all(pool)
        .await
        .context("Failed to fetch subfolders")?;

    Ok(rows.into_iter().map(FolderWithCounts::from).collect())
}

/// Move downloads to a folder.
#[tracing::instrument(name = "Move downloads to folder", skip(pool))]
pub async fn move_downloads_to_folder(
    download_ids: &[Uuid],
    folder_id: Option<Uuid>,
    user_id: &UserId,
    pool: &PgPool,
) -> Result<i64> {
    // Verify folder belongs to user if specified
    if let Some(fid) = folder_id {
        let _ = get_user_folder_by_id(fid, user_id, pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Folder not found"))?;
    }

    let result = sqlx::query(
        r"
        UPDATE downloads
        SET folder_id = $1, updated_at = NOW()
        WHERE id = ANY($2) AND user_id = $3
        ",
    )
    .bind(folder_id)
    .bind(download_ids)
    .bind(user_id.0)
    .execute(pool)
    .await
    .context("Failed to move downloads to folder")?;

    #[allow(clippy::cast_possible_wrap)]
    let count = result.rows_affected() as i64;
    Ok(count)
}

/// Helper row type for folder with counts.
#[derive(Debug, sqlx::FromRow)]
struct FolderWithCountsRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub path: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub subfolder_count: i64,
    pub download_count: i64,
}

impl From<FolderWithCountsRow> for FolderWithCounts {
    fn from(row: FolderWithCountsRow) -> Self {
        Self {
            folder: Folder {
                id: row.id,
                user_id: row.user_id,
                parent_id: row.parent_id,
                name: row.name,
                path: row.path,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
            subfolder_count: row.subfolder_count,
            download_count: row.download_count,
        }
    }
}
