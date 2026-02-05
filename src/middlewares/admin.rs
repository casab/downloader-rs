//! Admin authorization middleware.

use crate::middlewares::UserId;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::{Error, HttpMessage, HttpResponse, web};
use sqlx::PgPool;

/// Middleware that rejects non-admin users with a 403 Forbidden response.
pub async fn require_admin(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    // Get user_id from request extensions (set by auth middleware)
    let user_id = req.extensions().get::<UserId>().copied();

    let Some(user_id) = user_id else {
        let response = HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "Authentication required"}));
        return Ok(req.into_response(response).map_into_right_body());
    };

    // Check if user is admin
    let pool = req.app_data::<web::Data<PgPool>>().cloned();

    let Some(pool) = pool else {
        let response = HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "Internal server error"}));
        return Ok(req.into_response(response).map_into_right_body());
    };

    let is_admin = match sqlx::query_scalar::<_, bool>("SELECT is_admin FROM users WHERE id = $1")
        .bind(user_id.0)
        .fetch_optional(pool.as_ref())
        .await
    {
        Ok(Some(val)) => val,
        Ok(None) => false, // User not found
        Err(e) => {
            tracing::error!("Database error checking admin status: {}", e);
            let response = HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Internal server error"}));
            return Ok(req.into_response(response).map_into_right_body());
        },
    };

    if !is_admin {
        let response =
            HttpResponse::Forbidden().json(serde_json::json!({"error": "Admin access required"}));
        return Ok(req.into_response(response).map_into_right_body());
    }

    next.call(req)
        .await
        .map(ServiceResponse::map_into_left_body)
}
