use crate::clients::S3Client;
use crate::middlewares::UserId;
use crate::models::{DownloadProgress, DownloadQueryParams, DownloadStatus};
use crate::repository::{
    cancel_download as repo_cancel, create_download, get_download_by_id,
    get_downloads_paginated, get_user_download_by_id, pause_download as repo_pause,
    resume_download as repo_resume, retry_download as repo_retry, update_download_status,
};
use crate::utils::{download_file, e400, e404, e500};
use actix_web::{HttpResponse, web};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct Parameters {
    url: String,
}

#[tracing::instrument(name = "Download the given url to a file", skip(parameters, pool))]
pub async fn download(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    s3_client: web::Data<Option<S3Client>>,
) -> Result<HttpResponse, actix_web::Error> {
    let file_link = &parameters.url;
    let download = create_download(file_link, &user_id.into_inner(), None, None, &pool)
        .await
        .map_err(e500)?;

    match download_file(file_link, s3_client.get_ref().clone()).await {
        Ok(file_path) => {
            let updated = update_download_status(
                download.id,
                DownloadStatus::Completed,
                Some(file_path),
                None,
                &pool,
            )
            .await
            .map_err(e500)?;
            return Ok(HttpResponse::Ok().json(updated));
        },
        Err(err) => {
            tracing::error!(error = ?err, download_id = download.id.to_string(), "Failed to download the file");
            update_download_status(
                download.id,
                DownloadStatus::Failed,
                None,
                Some(err.to_string()),
                &pool,
            )
            .await
            .map_err(e500)?;
            use manic::ManicError;
            return match err {
                ManicError::NotFound => Err(e404("Failed to find the file")),
                _ => Err(e500("Failed to download the file")),
            };
        },
    }
}

#[tracing::instrument(name = "Get a download with id", skip(parameters, pool))]
pub async fn get_download(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let download_id = parameters.into_inner();
    let download = get_download_by_id(download_id, &pool).await.map_err(e500)?;
    match download {
        Some(download) => {
            if download.user_id != user_id.0 {
                return Err(e404("Download not found"));
            }
            Ok(HttpResponse::Ok().json(download))
        },
        None => Err(e404("Download not found")),
    }
}

#[tracing::instrument(name = "Get all downloads", skip(pool, query))]
pub async fn get_downloads(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    query: web::Query<DownloadQueryParams>,
) -> Result<HttpResponse, actix_web::Error> {
    let query_params = query.into_inner();
    let pagination = query_params.pagination();
    let filter = query_params.filter();
    let sort = query_params.sorting();

    let response = get_downloads_paginated(
        &pool,
        &user_id.into_inner(),
        &pagination,
        &filter,
        &sort,
    )
    .await
    .map_err(e500)?;

    Ok(HttpResponse::Ok().json(response))
}

/// Get download progress information.
#[tracing::instrument(name = "Get download progress", skip(pool))]
pub async fn get_progress(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let download_id = parameters.into_inner();
    let user_id = user_id.into_inner();

    let download = get_user_download_by_id(download_id, &user_id, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("Download not found"))?;

    let progress = DownloadProgress::from_download(&download);
    Ok(HttpResponse::Ok().json(progress))
}

/// Pause an in-progress download.
#[tracing::instrument(name = "Pause download", skip(pool))]
pub async fn pause_download(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let download_id = parameters.into_inner();
    let user_id = user_id.into_inner();

    // First check ownership and current state
    let download = get_user_download_by_id(download_id, &user_id, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("Download not found"))?;

    if !download.can_pause() {
        return Err(e400(format!(
            "Cannot pause download in {} state",
            download.status
        )));
    }

    let updated = repo_pause(download_id, &pool).await.map_err(e500)?;
    Ok(HttpResponse::Ok().json(updated))
}

/// Resume a paused download.
#[tracing::instrument(name = "Resume download", skip(pool))]
pub async fn resume_download(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let download_id = parameters.into_inner();
    let user_id = user_id.into_inner();

    let download = get_user_download_by_id(download_id, &user_id, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("Download not found"))?;

    if !download.can_resume() {
        return Err(e400(format!(
            "Cannot resume download in {} state",
            download.status
        )));
    }

    let updated = repo_resume(download_id, &pool).await.map_err(e500)?;
    Ok(HttpResponse::Ok().json(updated))
}

/// Retry a failed download.
#[tracing::instrument(name = "Retry download", skip(pool))]
pub async fn retry_download(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let download_id = parameters.into_inner();
    let user_id = user_id.into_inner();

    let download = get_user_download_by_id(download_id, &user_id, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("Download not found"))?;

    if !download.can_retry() {
        return Err(e400(format!(
            "Cannot retry download: status is {} (retry count: {}/{})",
            download.status, download.retry_count, download.max_retries
        )));
    }

    let updated = repo_retry(download_id, &pool).await.map_err(e500)?;
    Ok(HttpResponse::Ok().json(updated))
}

/// Cancel a download.
#[tracing::instrument(name = "Cancel download", skip(pool))]
pub async fn cancel_download(
    parameters: web::Path<uuid::Uuid>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let download_id = parameters.into_inner();
    let user_id = user_id.into_inner();

    let download = get_user_download_by_id(download_id, &user_id, &pool)
        .await
        .map_err(e500)?
        .ok_or_else(|| e404("Download not found"))?;

    if !download.can_cancel() {
        return Err(e400(format!(
            "Cannot cancel download in {} state",
            download.status
        )));
    }

    let updated = repo_cancel(download_id, &pool).await.map_err(e500)?;
    Ok(HttpResponse::Ok().json(updated))
}
