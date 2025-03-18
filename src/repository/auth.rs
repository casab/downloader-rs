use crate::models::User;
use crate::telemetry::spawn_blocking_with_tracing;
use crate::utils::compute_password_hash;
use secrecy::{ExposeSecret, SecretString};
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
    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3) returning *;
        "#,
        Uuid::new_v4(),
        email,
        password_hash.expose_secret(),
    )
    .fetch_one(pool)
    .await
    .context("Failed to insert user in the database.")?;
    Ok(user.id)
}

#[tracing::instrument(name = "Get stored credentials", skip(email, pool))]
pub async fn get_stored_credentials(
    email: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, SecretString)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT id, password_hash
        FROM users
        WHERE email = $1
        "#,
        email,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to validate auth credentials.")?
    .map(|row| (row.id, SecretString::new(row.password_hash.into())));
    Ok(row)
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
    sqlx::query!(
        r#"
        UPDATE users
        SET password_hash = $1
        WHERE id = $2
        "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to change user's password in the database.")?;
    Ok(())
}
