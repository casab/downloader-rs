//! Admin routes for user management, stats, and audit logs.

use crate::middlewares::UserId;
use crate::models::{AdminUpdateUserRequest, AuditLogQuery, CreateAuditLog};
use crate::repository;
use crate::utils::e500;
use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

/// GET /api/v1/admin/stats - Get system statistics.
#[tracing::instrument(name = "Get admin stats", skip(pool))]
pub async fn get_admin_stats(
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let stats = repository::get_admin_stats(&pool)
        .await
        .map_err(e500)?;

    Ok(HttpResponse::Ok().json(stats))
}

/// GET /api/v1/admin/users - List all users.
#[tracing::instrument(name = "Admin list users", skip(pool))]
pub async fn admin_list_users(
    pool: web::Data<PgPool>,
    query: web::Query<PaginationQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let limit = query.per_page.unwrap_or(50).min(100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    let users = repository::list_all_users(limit, offset, &pool)
        .await
        .map_err(e500)?;

    let total = repository::count_users(&pool)
        .await
        .map_err(e500)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "data": users,
        "total": total,
        "page": page,
        "per_page": limit,
    })))
}

/// GET /api/v1/admin/users/{id} - Get user details.
#[tracing::instrument(name = "Admin get user", skip(pool))]
pub async fn admin_get_user(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = path.into_inner();

    let user = repository::get_user_by_id_admin(user_id, &pool)
        .await
        .map_err(e500)?;

    match user {
        Some(u) => Ok(HttpResponse::Ok().json(u)),
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "User not found"}))),
    }
}

/// PATCH /api/v1/admin/users/{id} - Update user.
#[tracing::instrument(name = "Admin update user", skip(pool, req))]
pub async fn admin_update_user(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<AdminUpdateUserRequest>,
    user_id: web::ReqData<UserId>,
    req: HttpRequest,
) -> Result<HttpResponse, actix_web::Error> {
    let target_user_id = path.into_inner();
    let admin_id = user_id.into_inner();

    // Get old values for audit
    let old_user = repository::get_user_by_id_admin(target_user_id, &pool)
        .await
        .map_err(e500)?;

    repository::admin_update_user(target_user_id, &body, &pool)
        .await
        .map_err(e500)?;

    // Create audit log
    let audit = CreateAuditLog {
        user_id: Some(admin_id.0),
        action: "update".to_string(),
        resource_type: "user".to_string(),
        resource_id: Some(target_user_id),
        old_values: old_user.map(|u| serde_json::json!({"is_admin": u.is_admin, "email_verified": u.email_verified})),
        new_values: Some(serde_json::to_value(&*body).unwrap_or_default()),
        ip_address: req.peer_addr().map(|a| a.ip().to_string()),
        user_agent: req
            .headers()
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(String::from),
    };

    // Best-effort audit logging
    let _ = repository::create_audit_log(&audit, &pool).await;

    let updated = repository::get_user_by_id_admin(target_user_id, &pool)
        .await
        .map_err(e500)?;

    match updated {
        Some(u) => Ok(HttpResponse::Ok().json(u)),
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({"error": "User not found"}))),
    }
}

/// DELETE /api/v1/admin/users/{id} - Delete user.
#[tracing::instrument(name = "Admin delete user", skip(pool, req))]
pub async fn admin_delete_user(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    user_id: web::ReqData<UserId>,
    req: HttpRequest,
) -> Result<HttpResponse, actix_web::Error> {
    let target_user_id = path.into_inner();
    let admin_id = user_id.into_inner();

    // Don't allow self-deletion
    if admin_id.0 == target_user_id {
        return Ok(HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Cannot delete yourself"})));
    }

    repository::admin_delete_user(target_user_id, &pool)
        .await
        .map_err(e500)?;

    // Audit log
    let audit = CreateAuditLog {
        user_id: Some(admin_id.0),
        action: "delete".to_string(),
        resource_type: "user".to_string(),
        resource_id: Some(target_user_id),
        old_values: None,
        new_values: None,
        ip_address: req.peer_addr().map(|a| a.ip().to_string()),
        user_agent: req
            .headers()
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(String::from),
    };
    let _ = repository::create_audit_log(&audit, &pool).await;

    Ok(HttpResponse::NoContent().finish())
}

/// GET /api/v1/admin/audit-logs - List audit logs.
#[tracing::instrument(name = "List audit logs", skip(pool))]
pub async fn list_audit_logs(
    pool: web::Data<PgPool>,
    query: web::Query<AuditLogQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let logs = repository::list_audit_logs(&query, &pool)
        .await
        .map_err(e500)?;

    let total = repository::count_audit_logs(&query, &pool)
        .await
        .map_err(e500)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "data": logs,
        "total": total,
        "page": query.page.unwrap_or(1),
        "per_page": query.limit(),
    })))
}

/// Admin download listing row.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
struct AdminDownloadRow {
    id: Uuid,
    url: String,
    status: String,
    file_path: Option<String>,
    user_id: Uuid,
    bytes_downloaded: i64,
    total_bytes: Option<i64>,
    filename: Option<String>,
    error_message: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/v1/admin/downloads - List all downloads (admin view).
#[tracing::instrument(name = "Admin list downloads", skip(pool))]
pub async fn admin_list_downloads(
    pool: web::Data<PgPool>,
    query: web::Query<PaginationQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let limit = query.per_page.unwrap_or(50).min(100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    let downloads = sqlx::query_as::<_, AdminDownloadRow>(
        "SELECT id, url, status, file_path, user_id, bytes_downloaded, total_bytes, \
         filename, error_message, created_at, updated_at \
         FROM downloads ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool.get_ref())
    .await
    .map_err(e500)?;

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM downloads")
        .fetch_one(pool.get_ref())
        .await
        .unwrap_or(0);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "data": downloads,
        "total": total,
        "page": page,
        "per_page": limit,
    })))
}

/// GET /metrics - Prometheus metrics endpoint.
pub async fn metrics_handler() -> HttpResponse {
    let registry = prometheus_client::registry::Registry::default();
    let output = crate::metrics::encode_prometheus_metrics(&registry);

    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(output)
}

/// Simple pagination query params.
#[derive(Debug, serde::Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
