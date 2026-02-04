//! Token repository for managing verification tokens.

use crate::models::{Token, TokenType};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Create a new token.
#[tracing::instrument(name = "Create token", skip(pool, token_hash))]
pub async fn create_token(
    user_id: Uuid,
    token_hash: &str,
    token_type: TokenType,
    expires_at: DateTime<Utc>,
    metadata: Option<serde_json::Value>,
    pool: &PgPool,
) -> Result<Token> {
    let token = sqlx::query_as::<_, Token>(
        r#"
        INSERT INTO tokens (user_id, token_hash, token_type, expires_at, metadata)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, token_hash, token_type, expires_at, used_at, created_at, metadata
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(token_type)
    .bind(expires_at)
    .bind(metadata)
    .fetch_one(pool)
    .await
    .context("Failed to create token")?;

    Ok(token)
}

/// Find a valid token by its hash (not expired, not used).
#[tracing::instrument(name = "Find valid token by hash", skip(pool, token_hash))]
pub async fn find_valid_token_by_hash(
    token_hash: &str,
    token_type: TokenType,
    pool: &PgPool,
) -> Result<Option<Token>> {
    let token = sqlx::query_as::<_, Token>(
        r#"
        SELECT id, user_id, token_hash, token_type, expires_at, used_at, created_at, metadata
        FROM tokens
        WHERE token_hash = $1
          AND token_type = $2
          AND used_at IS NULL
          AND expires_at > NOW()
        "#,
    )
    .bind(token_hash)
    .bind(token_type)
    .fetch_optional(pool)
    .await
    .context("Failed to find token")?;

    Ok(token)
}

/// Atomically find and claim a valid token (mark it as used in a single query).
/// This prevents race conditions where two concurrent requests use the same token.
#[tracing::instrument(name = "Claim token", skip(pool, token_hash))]
pub async fn claim_token(
    token_hash: &str,
    token_type: TokenType,
    pool: &PgPool,
) -> Result<Option<Token>> {
    let token = sqlx::query_as::<_, Token>(
        r#"
        UPDATE tokens
        SET used_at = NOW()
        WHERE id = (
            SELECT id FROM tokens
            WHERE token_hash = $1
              AND token_type = $2
              AND used_at IS NULL
              AND expires_at > NOW()
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        )
        RETURNING id, user_id, token_hash, token_type, expires_at, used_at, created_at, metadata
        "#,
    )
    .bind(token_hash)
    .bind(token_type)
    .fetch_optional(pool)
    .await
    .context("Failed to claim token")?;

    Ok(token)
}

/// Mark a token as used.
#[tracing::instrument(name = "Mark token as used", skip(pool))]
pub async fn mark_token_used(token_id: Uuid, pool: &PgPool) -> Result<()> {
    let result = sqlx::query(
        r#"
        UPDATE tokens
        SET used_at = NOW()
        WHERE id = $1 AND used_at IS NULL
        "#,
    )
    .bind(token_id)
    .execute(pool)
    .await
    .context("Failed to mark token as used")?;

    if result.rows_affected() == 0 {
        anyhow::bail!("Token not found or already used");
    }

    Ok(())
}

/// Invalidate all unused tokens of a specific type for a user.
#[tracing::instrument(name = "Invalidate user tokens", skip(pool))]
pub async fn invalidate_user_tokens(
    user_id: Uuid,
    token_type: TokenType,
    pool: &PgPool,
) -> Result<u64> {
    let result = sqlx::query(
        r#"
        UPDATE tokens
        SET used_at = NOW()
        WHERE user_id = $1
          AND token_type = $2
          AND used_at IS NULL
        "#,
    )
    .bind(user_id)
    .bind(token_type)
    .execute(pool)
    .await
    .context("Failed to invalidate tokens")?;

    Ok(result.rows_affected())
}

/// Delete expired tokens (cleanup job).
#[tracing::instrument(name = "Delete expired tokens", skip(pool))]
pub async fn delete_expired_tokens(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query(
        r#"
        DELETE FROM tokens
        WHERE expires_at < NOW() - INTERVAL '7 days'
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to delete expired tokens")?;

    Ok(result.rows_affected())
}
