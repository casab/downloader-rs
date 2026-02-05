//! Admin repository functions.

use crate::models::{
    AdminStats, AdminUpdateUserRequest, AdminUserRow, DownloadStats, JobStats, StorageStats,
    UserStats,
};
use anyhow::{Context, Result};
use sqlx::PgPool;
use std::fmt::Write;
use uuid::Uuid;

/// Check if a user is an admin.
#[tracing::instrument(name = "Check admin status", skip(pool))]
pub async fn is_admin(user_id: Uuid, pool: &PgPool) -> Result<bool> {
    let result = sqlx::query_scalar::<_, bool>("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .context("Failed to check admin status")?;

    Ok(result.unwrap_or(false))
}

/// List all users (admin only).
#[tracing::instrument(name = "List all users", skip(pool))]
pub async fn list_all_users(limit: i64, offset: i64, pool: &PgPool) -> Result<Vec<AdminUserRow>> {
    let users = sqlx::query_as::<_, AdminUserRow>(
        r"
        SELECT id, email, display_name, is_admin, email_verified_at, created_at, updated_at
        FROM users
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        ",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .context("Failed to list users")?;

    Ok(users)
}

/// Get total user count.
#[tracing::instrument(name = "Count users", skip(pool))]
pub async fn count_users(pool: &PgPool) -> Result<i64> {
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .context("Failed to count users")?;

    Ok(count)
}

/// Get a single user by ID (admin view).
#[tracing::instrument(name = "Get user by ID (admin)", skip(pool))]
pub async fn get_user_by_id_admin(user_id: Uuid, pool: &PgPool) -> Result<Option<AdminUserRow>> {
    let user = sqlx::query_as::<_, AdminUserRow>(
        r"
        SELECT id, email, display_name, is_admin, email_verified_at, created_at, updated_at
        FROM users WHERE id = $1
        ",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get user")?;

    Ok(user)
}

/// Update user from admin panel.
#[tracing::instrument(name = "Admin update user", skip(pool))]
pub async fn admin_update_user(
    user_id: Uuid,
    req: &AdminUpdateUserRequest,
    pool: &PgPool,
) -> Result<()> {
    let mut query = String::from("UPDATE users SET updated_at = NOW()");
    let mut param_count = 0;

    if req.is_admin.is_some() {
        param_count += 1;
        let _ = write!(query, ", is_admin = ${param_count}");
    }
    if req.email_verified.is_some() {
        param_count += 1;
        let _ = write!(
            query,
            ", email_verified_at = CASE WHEN ${param_count} THEN NOW() ELSE NULL END"
        );
    }

    param_count += 1;
    let _ = write!(query, " WHERE id = ${param_count}");

    let mut q = sqlx::query(&query);

    if let Some(is_admin) = req.is_admin {
        q = q.bind(is_admin);
    }
    if let Some(email_verified) = req.email_verified {
        q = q.bind(email_verified);
    }

    q = q.bind(user_id);

    q.execute(pool).await.context("Failed to update user")?;

    Ok(())
}

/// Delete a user (admin only).
#[tracing::instrument(name = "Admin delete user", skip(pool))]
pub async fn admin_delete_user(user_id: Uuid, pool: &PgPool) -> Result<()> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .context("Failed to delete user")?;

    Ok(())
}

/// Get admin statistics.
#[tracing::instrument(name = "Get admin stats", skip(pool))]
pub async fn get_admin_stats(pool: &PgPool) -> Result<AdminStats> {
    // User stats
    let total_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let active_today = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE updated_at > NOW() - INTERVAL '1 day'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let active_this_week = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE updated_at > NOW() - INTERVAL '7 days'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let new_this_month = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE created_at > NOW() - INTERVAL '30 days'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Download stats
    let total_downloads = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM downloads")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let completed_downloads =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM downloads WHERE status = 'COMPLETED'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let failed_downloads =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM downloads WHERE status = 'FAILED'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let in_progress_downloads =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM downloads WHERE status = 'IN_PROGRESS'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let total_bytes = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(total_bytes), 0) FROM downloads WHERE status = 'COMPLETED'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Storage stats
    let total_files =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM downloads WHERE status = 'COMPLETED'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    Ok(AdminStats {
        users: UserStats {
            total: total_users,
            active_today,
            active_this_week,
            new_this_month,
        },
        downloads: DownloadStats {
            total: total_downloads,
            completed: completed_downloads,
            failed: failed_downloads,
            in_progress: in_progress_downloads,
            total_bytes,
        },
        storage: StorageStats {
            total_bytes_used: total_bytes,
            total_files,
        },
        jobs: JobStats {
            pending: 0,
            completed: 0,
            failed: 0,
        },
    })
}
