use crate::clients::S3Client;
use crate::middlewares::UserId;
use crate::models::DownloadStatus;
use crate::repository::{
    create_download, get_all_downloads, get_download_by_id, update_download_status,
};
use crate::utils::{download_file, e404, e500};
use actix_web::{web, HttpResponse};
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
    let download = create_download(file_link, &user_id.into_inner(), &pool)
        .await
        .map_err(e500)?;

    match download_file(file_link, s3_client.get_ref().clone()).await {
        Ok(file_path) => {
            update_download_status(
                download.id,
                DownloadStatus::Completed,
                Some(file_path),
                &pool,
            )
            .await
            .map_err(e500)?;
            return Ok(HttpResponse::Ok().finish());
        }
        Err(err) => {
            tracing::error!(error = ?err, download_id = download.id.to_string(), "Failed to download the file");
            update_download_status(download.id, DownloadStatus::Failed, None, &pool)
                .await
                .map_err(e500)?;
            use manic::ManicError;
            return match err {
                ManicError::NotFound => Err(e404("Failed to find the file")),
                _ => Err(e500("Failed to download the file")),
            };
        }
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
        }
        None => Err(e404("Download not found")),
    }
}

#[tracing::instrument(name = "Get all downloads", skip(pool))]
pub async fn get_downloads(
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    let downloads = get_all_downloads(&pool, &user_id.into_inner())
        .await
        .map_err(e500)?;
    Ok(HttpResponse::Ok().json(downloads))
}
