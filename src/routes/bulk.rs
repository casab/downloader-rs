//! Bulk operations routes.

use crate::middlewares::UserId;
use crate::models::{
    BulkDeleteRequest, BulkMoveRequest, BulkOperationFailure, BulkOperationResponse,
    BulkTagRequest, BulkUntagRequest,
};
use crate::repository::{
    bulk_add_tags, bulk_remove_tags, get_user_download_by_id, move_downloads_to_folder,
};
use crate::utils::{e400, e500};
use actix_web::{HttpResponse, web};
use sqlx::PgPool;

const MAX_BULK_IDS: usize = 100;

/// Move multiple downloads to a folder.
#[tracing::instrument(name = "Bulk move downloads", skip(pool, body))]
pub async fn bulk_move(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    body: web::Json<BulkMoveRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let request = body.into_inner();

    // Validate
    if request.download_ids.is_empty() {
        return Err(e400("No downloads specified"));
    }
    if request.download_ids.len() > MAX_BULK_IDS {
        return Err(e400(format!("Cannot process more than {MAX_BULK_IDS} items at once")));
    }

    let moved = move_downloads_to_folder(
        &request.download_ids,
        request.folder_id,
        &user_id,
        &pool,
    )
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("not found") {
            e400(msg)
        } else {
            e500(e)
        }
    })?;

    #[allow(clippy::cast_possible_wrap)]
    let total = request.download_ids.len() as i64;
    let response = if moved == total {
        // All succeeded — safe to report all IDs
        BulkOperationResponse::all_success(request.download_ids)
    } else {
        // Partial success — we don't know which specific IDs failed
        BulkOperationResponse {
            success_count: moved,
            failure_count: total - moved,
            successful_ids: Vec::new(),
            failed_items: Vec::new(),
        }
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Add tags to multiple downloads.
#[tracing::instrument(name = "Bulk tag downloads", skip(pool, body))]
pub async fn bulk_tag(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    body: web::Json<BulkTagRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let request = body.into_inner();

    // Validate
    if request.download_ids.is_empty() {
        return Err(e400("No downloads specified"));
    }
    if request.tag_ids.is_empty() {
        return Err(e400("No tags specified"));
    }
    if request.download_ids.len() > MAX_BULK_IDS {
        return Err(e400(format!("Cannot process more than {MAX_BULK_IDS} items at once")));
    }

    let added = bulk_add_tags(&request.download_ids, &request.tag_ids, &user_id, &pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not found") {
                e400(msg)
            } else {
                e500(e)
            }
        })?;

    let response = BulkOperationResponse {
        success_count: added,
        failure_count: 0,
        successful_ids: request.download_ids,
        failed_items: Vec::new(),
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Remove tags from multiple downloads.
#[tracing::instrument(name = "Bulk untag downloads", skip(pool, body))]
pub async fn bulk_untag(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    body: web::Json<BulkUntagRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let request = body.into_inner();

    // Validate
    if request.download_ids.is_empty() {
        return Err(e400("No downloads specified"));
    }
    if request.tag_ids.is_empty() {
        return Err(e400("No tags specified"));
    }
    if request.download_ids.len() > MAX_BULK_IDS {
        return Err(e400(format!("Cannot process more than {MAX_BULK_IDS} items at once")));
    }

    let removed = bulk_remove_tags(&request.download_ids, &request.tag_ids, &user_id, &pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not found") {
                e400(msg)
            } else {
                e500(e)
            }
        })?;

    let response = BulkOperationResponse {
        success_count: removed,
        failure_count: 0,
        successful_ids: request.download_ids,
        failed_items: Vec::new(),
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Delete multiple downloads.
#[tracing::instrument(name = "Bulk delete downloads", skip(pool, body))]
pub async fn bulk_delete(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    body: web::Json<BulkDeleteRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    let request = body.into_inner();

    // Validate
    if request.download_ids.is_empty() {
        return Err(e400("No downloads specified"));
    }
    if request.download_ids.len() > MAX_BULK_IDS {
        return Err(e400(format!("Cannot process more than {MAX_BULK_IDS} items at once")));
    }

    let mut successful_ids = Vec::new();
    let mut failed_items = Vec::new();

    for download_id in request.download_ids {
        // Check ownership
        match get_user_download_by_id(download_id, &user_id, &pool).await {
            Ok(Some(_)) => {
                // Delete the download
                match sqlx::query("DELETE FROM downloads WHERE id = $1 AND user_id = $2")
                    .bind(download_id)
                    .bind(user_id.0)
                    .execute(pool.as_ref())
                    .await
                {
                    Ok(_) => successful_ids.push(download_id),
                    Err(e) => failed_items.push(BulkOperationFailure::new(
                        download_id,
                        format!("Delete failed: {e}"),
                    )),
                }
            }
            Ok(None) => {
                failed_items.push(BulkOperationFailure::new(download_id, "Download not found"));
            }
            Err(e) => {
                failed_items.push(BulkOperationFailure::new(
                    download_id,
                    format!("Error checking download: {e}"),
                ));
            }
        }
    }

    #[allow(clippy::cast_possible_wrap)]
    let response = BulkOperationResponse {
        success_count: successful_ids.len() as i64,
        failure_count: failed_items.len() as i64,
        successful_ids,
        failed_items,
    };

    Ok(HttpResponse::Ok().json(response))
}
