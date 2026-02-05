//! User repository for profile management operations.

use crate::models::{UpdateProfileRequest, User, UserRow};
use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;

/// Get a user by ID (excluding soft-deleted users).
#[tracing::instrument(name = "Get user by ID", skip(pool))]
pub async fn get_user_by_id(user_id: Uuid, pool: &PgPool) -> Result<Option<User>> {
    let row = sqlx::query_as::<_, UserRow>(
        r"
        SELECT id, email, password_hash, display_name, avatar_url, bio, timezone,
               email_verified_at, deleted_at, created_at, updated_at
        FROM users
        WHERE id = $1 AND deleted_at IS NULL
        ",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch user by ID")?;

    Ok(row.map(User::from))
}

/// Get a user by email (excluding soft-deleted users).
#[tracing::instrument(name = "Get user by email", skip(pool))]
pub async fn get_user_by_email(email: &str, pool: &PgPool) -> Result<Option<User>> {
    let row = sqlx::query_as::<_, UserRow>(
        r"
        SELECT id, email, password_hash, display_name, avatar_url, bio, timezone,
               email_verified_at, deleted_at, created_at, updated_at
        FROM users
        WHERE email = $1 AND deleted_at IS NULL
        ",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch user by email")?;

    Ok(row.map(User::from))
}

/// Update user profile fields.
#[tracing::instrument(name = "Update user profile", skip(pool, request))]
pub async fn update_user_profile(
    user_id: Uuid,
    request: &UpdateProfileRequest,
    pool: &PgPool,
) -> Result<User> {
    let row = sqlx::query_as::<_, UserRow>(
        r"
        UPDATE users
        SET display_name = COALESCE($2, display_name),
            avatar_url = COALESCE($3, avatar_url),
            bio = COALESCE($4, bio),
            timezone = COALESCE($5, timezone),
            updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING id, email, password_hash, display_name, avatar_url, bio, timezone,
                  email_verified_at, deleted_at, created_at, updated_at
        ",
    )
    .bind(user_id)
    .bind(&request.display_name)
    .bind(&request.avatar_url)
    .bind(&request.bio)
    .bind(&request.timezone)
    .fetch_one(pool)
    .await
    .context("Failed to update user profile")?;

    Ok(User::from(row))
}

/// Soft delete a user account.
#[tracing::instrument(name = "Soft delete user", skip(pool))]
pub async fn soft_delete_user(user_id: Uuid, pool: &PgPool) -> Result<()> {
    let result = sqlx::query(
        r"
        UPDATE users
        SET deleted_at = NOW(),
            updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        ",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .context("Failed to soft delete user")?;

    if result.rows_affected() == 0 {
        anyhow::bail!("User not found or already deleted");
    }

    Ok(())
}

/// Mark user email as verified.
#[tracing::instrument(name = "Verify user email", skip(pool))]
pub async fn mark_email_verified(user_id: Uuid, pool: &PgPool) -> Result<()> {
    sqlx::query(
        r"
        UPDATE users
        SET email_verified_at = NOW(),
            updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        ",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .context("Failed to mark email as verified")?;

    Ok(())
}

/// Update user's password hash.
#[tracing::instrument(name = "Update user password hash", skip(pool, password_hash))]
pub async fn update_password_hash(
    user_id: Uuid,
    password_hash: &str,
    pool: &PgPool,
) -> Result<()> {
    sqlx::query(
        r"
        UPDATE users
        SET password_hash = $2,
            updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        ",
    )
    .bind(user_id)
    .bind(password_hash)
    .execute(pool)
    .await
    .context("Failed to update password hash")?;

    Ok(())
}
