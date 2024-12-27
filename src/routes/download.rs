use crate::utils::download_file;
use actix_web::{web, HttpResponse};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct Parameters {
    url: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, _pool))]
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
