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
use std::net::IpAddr;

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

    // SSRF protection: validate URL before processing
    validate_download_url(file_link).await.map_err(e400)?;

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

// =============================================================================
// SSRF Protection
// =============================================================================

/// Validate that a download URL is safe (not targeting internal/private networks).
async fn validate_download_url(raw_url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(raw_url).map_err(|e| format!("Invalid URL: {e}"))?;

    // Only allow http and https schemes
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(format!(
                "Unsupported URL scheme '{scheme}': only HTTP and HTTPS are allowed"
            ))
        }
    }

    let host = parsed.host().ok_or("URL must have a host")?;

    match host {
        url::Host::Ipv4(ip) => {
            if is_private_ipv4(&ip) {
                return Err(
                    "Access to internal/private network addresses is not allowed".to_string(),
                );
            }
        }
        url::Host::Ipv6(ip) => {
            if is_private_ipv6(&ip) {
                return Err(
                    "Access to internal/private network addresses is not allowed".to_string(),
                );
            }
        }
        url::Host::Domain(domain) => {
            // Resolve hostname and check all returned IPs
            let port = parsed.port_or_known_default().unwrap_or(80);
            let addrs = tokio::net::lookup_host(format!("{domain}:{port}"))
                .await
                .map_err(|e| format!("Failed to resolve hostname: {e}"))?;

            for addr in addrs {
                match addr.ip() {
                    IpAddr::V4(ip) if is_private_ipv4(&ip) => {
                        return Err(
                            "URL resolves to an internal/private network address".to_string(),
                        );
                    }
                    IpAddr::V6(ip) if is_private_ipv6(&ip) => {
                        return Err(
                            "URL resolves to an internal/private network address".to_string(),
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn is_private_ipv4(ip: &std::net::Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_unspecified()
        // 100.64.0.0/10 (Carrier-grade NAT)
        || (ip.octets()[0] == 100 && (ip.octets()[1] & 0xC0) == 64)
}

fn is_private_ipv6(ip: &std::net::Ipv6Addr) -> bool {
    ip.is_loopback()
        || ip.is_unspecified()
        // Check for IPv4-mapped IPv6 addresses (e.g., ::ffff:127.0.0.1)
        || ip.to_ipv4_mapped().is_some_and(|ipv4| is_private_ipv4(&ipv4))
}
