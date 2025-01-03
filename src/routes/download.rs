use crate::models::Download;
use crate::utils::download_file;
use actix_web::{web, HttpResponse};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct Parameters {
    url: String,
}

#[tracing::instrument(name = "Download the given url to a file", skip(parameters, _pool))]
pub async fn download(
    parameters: web::Query<Parameters>,
    _pool: web::Data<PgPool>,
) -> HttpResponse {
    let file_link = &parameters.url;
    if download_file(file_link).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    return HttpResponse::Ok().finish();
}

#[tracing::instrument(name = "Get a download with id", skip(parameters, pool))]
pub async fn get_download(parameters: web::Path<i64>, pool: web::Data<PgPool>) -> HttpResponse {
    let id = parameters.into_inner();
    let download = match sqlx::query_as!(Download, "SELECT * FROM downloads WHERE id = $1", id)
        .fetch_one(&**pool)
        .await
    {
        Ok(download) => download,
        Err(_) => return HttpResponse::NotFound().finish(),
    };
    HttpResponse::Ok().json(download)
}

#[tracing::instrument(name = "Get all downloads", skip(pool))]
pub async fn get_downloads(pool: web::Data<PgPool>) -> HttpResponse {
    let downloads = match sqlx::query_as!(Download, "SELECT * FROM downloads")
        .fetch_all(&**pool)
        .await
    {
        Ok(downloads) => downloads,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    HttpResponse::Ok().json(downloads)
}
