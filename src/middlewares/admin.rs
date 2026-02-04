//! Admin authorization middleware.

use crate::middlewares::UserId;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::{web, Error, HttpMessage, HttpResponse};
use sqlx::PgPool;

/// Middleware that rejects non-admin users with a 403 Forbidden response.
pub async fn require_admin(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    // Get user_id from request extensions (set by auth middleware)
    let user_id = req
        .extensions()
        .get::<UserId>()
        .cloned();

    let user_id = match user_id {
        Some(uid) => uid,
        None => {
            let response = HttpResponse::Unauthorized()
                .json(serde_json::json!({"error": "Authentication required"}));
            return Ok(req.into_response(response).map_into_right_body());
        }
    };

    // Check if user is admin
    let pool = req
        .app_data::<web::Data<PgPool>>()
        .cloned();

    let pool = match pool {
        Some(p) => p,
        None => {
            let response = HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Internal server error"}));
            return Ok(req.into_response(response).map_into_right_body());
        }
    };

    let is_admin = sqlx::query_scalar::<_, bool>(
        "SELECT is_admin FROM users WHERE id = $1",
    )
    .bind(user_id.0)
    .fetch_optional(pool.as_ref())
    .await
    .unwrap_or(None)
    .unwrap_or(false);

    if !is_admin {
        let response = HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Admin access required"}));
        return Ok(req.into_response(response).map_into_right_body());
    }

    next.call(req).await.map(ServiceResponse::map_into_left_body)
}
