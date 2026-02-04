//! User profile management endpoints.

use crate::middlewares::UserId;
use crate::models::{ChangePasswordRequest, UpdateProfileRequest, UserProfile};
use crate::repository::{get_user_by_id, soft_delete_user, update_password_hash, update_user_profile};
use crate::telemetry::spawn_blocking_with_tracing;
use crate::utils::{compute_password_hash, e400, e404, e500, verify_password_hash};
use actix_web::{HttpResponse, web};
use secrecy::SecretString;
use sqlx::PgPool;

/// GET /api/v1/me - Get current user profile.
#[tracing::instrument(name = "Get current user profile", skip(pool))]
pub async fn get_current_user(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user = get_user_by_id(user_id.0, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("User not found"))?;

    let profile: UserProfile = user.into();
    Ok(HttpResponse::Ok().json(profile))
}

/// PATCH /api/v1/me - Update current user profile.
#[tracing::instrument(name = "Update user profile", skip(pool, body))]
pub async fn update_profile(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    body: web::Json<UpdateProfileRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let request = body.into_inner();

    if !request.has_updates() {
        return Err(e400("No fields provided for update"));
    }

    // Validate timezone if provided
    if let Some(ref tz) = request.timezone {
        if !is_valid_timezone(tz) {
            return Err(e400("Invalid timezone"));
        }
    }

    // Validate avatar URL if provided
    if let Some(ref url) = request.avatar_url {
        if !url.is_empty() && !is_valid_url(url) {
            return Err(e400("Invalid avatar URL"));
        }
    }

    let user = update_user_profile(user_id.0, &request, &pool)
        .await
        .map_err(e500)?;

    let profile: UserProfile = user.into();
    Ok(HttpResponse::Ok().json(profile))
}

/// DELETE /api/v1/me - Soft delete current user account.
#[tracing::instrument(name = "Delete user account", skip(pool))]
pub async fn delete_account(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    soft_delete_user(user_id.0, &pool).await.map_err(e500)?;

    Ok(HttpResponse::NoContent().finish())
}

/// POST /api/v1/me/password - Change password.
#[tracing::instrument(name = "Change password", skip(pool, body))]
pub async fn change_password(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    body: web::Json<ChangePasswordRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let request = body.into_inner();

    // Validate new password
    if request.new_password.len() < 8 {
        return Err(e400("Password must be at least 8 characters"));
    }

    if request.new_password == request.current_password {
        return Err(e400("New password must be different from current password"));
    }

    // Get current user to verify current password
    let user = get_user_by_id(user_id.0, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("User not found"))?;

    // Verify current password
    let current_password = SecretString::new(request.current_password.into());
    let password_hash = user.password_hash.clone();

    spawn_blocking_with_tracing(move || verify_password_hash(password_hash, current_password))
        .await
        .map_err(e500)?
        .map_err(|_| e400("Current password is incorrect"))?;

    // Hash new password
    let new_password = SecretString::new(request.new_password.into());
    let new_hash = spawn_blocking_with_tracing(move || compute_password_hash(new_password))
        .await
        .map_err(e500)?
        .map_err(e500)?;

    // Update password in database
    use secrecy::ExposeSecret;
    update_password_hash(user_id.0, new_hash.expose_secret(), &pool)
        .await
        .map_err(e500)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Password changed successfully"
    })))
}

/// Check if a timezone string is valid (basic validation).
fn is_valid_timezone(tz: &str) -> bool {
    // Basic validation - could be enhanced with chrono-tz
    !tz.is_empty() && tz.len() <= 64 && tz.chars().all(|c| c.is_alphanumeric() || c == '/' || c == '_' || c == '-')
}

/// Check if a URL is valid (basic validation).
fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}
