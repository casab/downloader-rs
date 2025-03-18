use crate::repository::{create_user, get_stored_credentials};
use crate::session_state::TypedSession;
use crate::telemetry::spawn_blocking_with_tracing;
use crate::utils::{e401, e500, errors::AuthError, verify_password_hash};
use actix_web::{web, HttpResponse};

use anyhow::{Context, Result};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct Credentials {
    email: String,
    password: SecretString,
}

#[derive(Serialize)]
pub struct AuthResponse {
    user_id: uuid::Uuid,
}

#[tracing::instrument(
    skip(login_data, pool, session),
    fields(email=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    login_data: web::Json<Credentials>,
    pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    let credentials = Credentials {
        email: login_data.email.clone(),
        password: login_data.password.clone(),
    };

    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            session.renew();
            session.insert_user_id(user_id).map_err(|e| e401(e))?;
            Ok(HttpResponse::Ok().json(AuthResponse { user_id }))
        }
        Err(e) => Err(e401(e)),
    }
}

#[tracing::instrument(
    skip(register_data, pool, session),
    fields(email=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn register(
    register_data: web::Json<Credentials>,
    pool: web::Data<PgPool>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    let credentials = Credentials {
        email: register_data.email.clone(),
        password: register_data.password.clone(),
    };

    match create_user(credentials.email, credentials.password, &pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            session.renew();
            session.insert_user_id(user_id).map_err(|e| e401(e))?;
            Ok(HttpResponse::Ok().json(AuthResponse { user_id }))
        }
        Err(e) => Err(e500(e)),
    }
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = SecretString::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
gZiV/M1gPc22ElAH/Jh1Hw$\
CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .into(),
    );

    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&credentials.email, &pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")??;

    // If user_id is None, we are going to return AuthError no matter what
    // Calculation is only done to make the time difference non-existent
    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown email."))
        .map_err(AuthError::InvalidCredentials)
}
