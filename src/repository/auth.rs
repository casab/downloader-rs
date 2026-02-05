//! Authentication repository for user creation and credential validation.

use crate::telemetry::spawn_blocking_with_tracing;
use crate::utils::compute_password_hash;
use secrecy::{ExposeSecret, SecretString};
use sqlx::Row;
use uuid::Uuid;

use anyhow::{Context, Result};
use sqlx::PgPool;

#[tracing::instrument(name = "Create a new user", skip(password, pool))]
pub async fn create_user(
    email: String,
    password: SecretString,
    pool: &PgPool,
) -> Result<Uuid, anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("Failed to hash password.")?;
    let user_id = Uuid::new_v4();
    sqlx::query(
        r"
        INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)
        ",
    )
    .bind(user_id)
    .bind(&email)
    .bind(password_hash.expose_secret())
    .execute(pool)
    .await
    .context("Failed to insert user in the database.")?;
    Ok(user_id)
}

#[tracing::instrument(name = "Get stored credentials", skip(email, pool))]
pub async fn get_stored_credentials(
    email: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, SecretString)>, anyhow::Error> {
    let row = sqlx::query(
        r"
        SELECT id, password_hash
        FROM users
        WHERE email = $1 AND deleted_at IS NULL
        ",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to validate auth credentials.")?;

    Ok(row.map(|r| {
        let id: Uuid = r.get("id");
        let password_hash: String = r.get("password_hash");
        (id, SecretString::new(password_hash.into()))
    }))
}

#[tracing::instrument(name = "Change password", skip(password, pool))]
pub async fn change_password(
    user_id: Uuid,
    password: SecretString,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(move || compute_password_hash(password))
        .await?
        .context("Failed to hash password.")?;
    sqlx::query(
        r"
        UPDATE users
        SET password_hash = $1, updated_at = NOW()
        WHERE id = $2 AND deleted_at IS NULL
        ",
    )
    .bind(password_hash.expose_secret())
    .bind(user_id)
    .execute(pool)
    .await
    .context("Failed to change user's password in the database.")?;
    Ok(())
}
