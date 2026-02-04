//! Usage reporting routes.

use crate::middlewares::UserId;
use crate::repository::get_usage_summary;
use crate::utils::e500;
use actix_web::{HttpResponse, web};
use sqlx::PgPool;

/// Default storage quota (10 GB).
const DEFAULT_QUOTA_BYTES: i64 = 10 * 1024 * 1024 * 1024;

/// Get usage summary for the current user.
#[tracing::instrument(name = "Get usage", skip(pool))]
pub async fn get_usage(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();

    let summary = get_usage_summary(&user_id, DEFAULT_QUOTA_BYTES, &pool)
        .await
        .map_err(e500)?;

    Ok(HttpResponse::Ok().json(summary))
}
