//! Search routes.

use crate::middlewares::UserId;
use crate::models::SearchQuery;
use crate::repository::search;
use crate::utils::{e400, e500};
use actix_web::{HttpResponse, web};
use sqlx::PgPool;

/// Perform a full-text search.
#[tracing::instrument(name = "Search", skip(pool, query))]
pub async fn search_handler(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    query: web::Query<SearchQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let search_query = query.into_inner();

    // Validate search query
    if search_query.q.trim().is_empty() {
        return Err(e400("Search query cannot be empty"));
    }

    if search_query.q.len() > 500 {
        return Err(e400("Search query too long"));
    }

    let results = search(&search_query, &user_id, &pool)
        .await
        .map_err(e500)?;

    Ok(HttpResponse::Ok().json(results))
}
