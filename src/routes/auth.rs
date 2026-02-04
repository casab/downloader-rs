use crate::configuration::JwtSettings;
use crate::middlewares::auth::create_jwt_token;
use crate::models::{ForgotPasswordRequest, ResetPasswordRequest, TokenType, VerifyEmailRequest};
use crate::repository::{
    create_token, create_user, find_valid_token_by_hash, get_stored_credentials, get_user_by_email,
    get_user_by_id, invalidate_user_tokens, mark_email_verified, mark_token_used,
    update_password_hash,
};
use crate::session_state::TypedSession;
use crate::telemetry::spawn_blocking_with_tracing;
use crate::utils::{
    compute_password_hash, e400, e401, e404, e500, errors::AuthError, generate_token, hash_token,
    verify_password_hash,
};
use actix_web::{HttpResponse, web};

use anyhow::{Context, Result};
use chrono::Utc;
use secrecy::{ExposeSecret, SecretString};
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
    jwt: String,
}

#[tracing::instrument(
    skip(login_data, pool, session),
    fields(email=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    login_data: web::Json<Credentials>,
    pool: web::Data<PgPool>,
    jwt_settings: web::Data<JwtSettings>,
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
            session.insert_user_id(user_id).map_err(e401)?;
            let jwt = create_jwt_token(user_id, &jwt_settings).map_err(e500)?;
            Ok(HttpResponse::Ok().json(AuthResponse { user_id, jwt }))
        },
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
    jwt_settings: web::Data<JwtSettings>,
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
            session.insert_user_id(user_id).map_err(e401)?;
            let jwt = create_jwt_token(user_id, &jwt_settings).map_err(e500)?;
            Ok(HttpResponse::Ok().json(AuthResponse { user_id, jwt }))
        },
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
        get_stored_credentials(&credentials.email, pool).await?
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

// =============================================================================
// Password Reset Endpoints
// =============================================================================

/// POST /api/v1/auth/forgot-password - Request password reset.
#[tracing::instrument(name = "Request password reset", skip(pool, body))]
pub async fn forgot_password(
    pool: web::Data<PgPool>,
    body: web::Json<ForgotPasswordRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let email = body.into_inner().email;

    // Always return success to prevent email enumeration
    // But only create token if user exists
    if let Ok(Some(user)) = get_user_by_email(&email, &pool).await {
        // Invalidate any existing password reset tokens
        let _ = invalidate_user_tokens(user.id, TokenType::PasswordReset, &pool).await;

        // Generate new token
        let token = generate_token();
        let token_hash = hash_token(&token);
        let expires_at = Utc::now() + TokenType::PasswordReset.default_expiration();

        // Store token
        if let Err(e) = create_token(
            user.id,
            &token_hash,
            TokenType::PasswordReset,
            expires_at,
            None,
            &pool,
        )
        .await
        {
            tracing::error!("Failed to create password reset token: {:?}", e);
        } else {
            // In production, send email with reset link containing the token
            // For now, log the token (REMOVE IN PRODUCTION)
            tracing::info!(
                "Password reset token for {}: {} (expires: {})",
                email,
                token,
                expires_at
            );
        }
    }

    // Always return success
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "If an account exists with this email, a password reset link has been sent."
    })))
}

/// POST /api/v1/auth/reset-password - Reset password with token.
#[tracing::instrument(name = "Reset password", skip(pool, body))]
pub async fn reset_password(
    pool: web::Data<PgPool>,
    body: web::Json<ResetPasswordRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let request = body.into_inner();

    // Validate new password
    if request.new_password.len() < 8 {
        return Err(e400("Password must be at least 8 characters"));
    }

    // Find and validate token
    let token_hash = hash_token(&request.token);
    let token = find_valid_token_by_hash(&token_hash, TokenType::PasswordReset, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e400("Invalid or expired token"))?;

    // Hash new password
    let new_password = SecretString::new(request.new_password.into());
    let new_hash = spawn_blocking_with_tracing(move || compute_password_hash(new_password))
        .await
        .map_err(e500)?
        .map_err(e500)?;

    // Update password
    update_password_hash(token.user_id, new_hash.expose_secret(), &pool)
        .await
        .map_err(e500)?;

    // Mark token as used
    mark_token_used(token.id, &pool).await.map_err(e500)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Password has been reset successfully"
    })))
}

// =============================================================================
// Email Verification Endpoints
// =============================================================================

/// POST /api/v1/auth/verify-email - Verify email with token.
#[tracing::instrument(name = "Verify email", skip(pool, body))]
pub async fn verify_email(
    pool: web::Data<PgPool>,
    body: web::Json<VerifyEmailRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let token_str = body.into_inner().token;

    // Find and validate token
    let token_hash = hash_token(&token_str);
    let token = find_valid_token_by_hash(&token_hash, TokenType::EmailVerification, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e400("Invalid or expired token"))?;

    // Mark email as verified
    mark_email_verified(token.user_id, &pool)
        .await
        .map_err(e500)?;

    // Mark token as used
    mark_token_used(token.id, &pool).await.map_err(e500)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Email verified successfully"
    })))
}

/// POST /api/v1/auth/resend-verification - Resend email verification.
#[tracing::instrument(name = "Resend email verification", skip(pool))]
pub async fn resend_verification(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<crate::middlewares::UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user = get_user_by_id(user_id.0, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("User not found"))?;

    if user.is_email_verified() {
        return Err(e400("Email is already verified"));
    }

    // Invalidate any existing verification tokens
    let _ = invalidate_user_tokens(user.id, TokenType::EmailVerification, &pool).await;

    // Generate new token
    let token = generate_token();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + TokenType::EmailVerification.default_expiration();

    // Store token
    create_token(
        user.id,
        &token_hash,
        TokenType::EmailVerification,
        expires_at,
        None,
        &pool,
    )
    .await
    .map_err(e500)?;

    // In production, send email with verification link
    // For now, log the token (REMOVE IN PRODUCTION)
    tracing::info!(
        "Email verification token for {}: {} (expires: {})",
        user.email,
        token,
        expires_at
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Verification email has been sent"
    })))
}
