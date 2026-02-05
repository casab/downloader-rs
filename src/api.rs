use crate::clients::get_s3_client;
use crate::clients::{StorageProvider, create_storage_provider};
use crate::configuration::{DatabaseSettings, JwtSettings, S3Settings, Settings};
use crate::events::{EventPublisher, LogEventPublisher};
use crate::middlewares::{
    RateLimitConfig, RateLimiter, rate_limit_middleware, reject_anonymous_users, require_admin,
};
use crate::routes::{
    // Auth & User
    cancel_download, change_password, delete_account, download, forgot_password, get_current_user,
    get_download, get_downloads, get_progress, health_check, login, pause_download, register,
    resend_verification, reset_password, resume_download, retry_download, update_profile,
    verify_email,
    // Folders
    create_folder, list_root_folders, get_folder, get_folder_subfolders, update_folder,
    move_folder, delete_folder,
    // Tags
    create_tag, list_tags, get_tag, update_tag, delete_tag, add_download_tags,
    remove_download_tag, get_tags_for_download,
    // Bulk operations
    bulk_move, bulk_tag, bulk_untag, bulk_delete,
    // Search
    search_handler,
    // Usage
    get_usage,
    // Health & Admin
    detailed_health_check, get_admin_stats, admin_list_users, admin_get_user,
    admin_update_user, admin_delete_user, list_audit_logs, admin_list_downloads,
    metrics_handler,
};
use crate::utils::error_handler;
use actix_session::{SessionMiddleware, storage::RedisSessionStore};
use actix_web::{
    App, HttpServer,
    cookie::Key,
    dev::Server,
    middleware::{NormalizePath, from_fn},
    web,
};
use secrecy::{ExposeSecret, SecretString};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::TcpListener;
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
    worker_pool: Option<crate::jobs::WorkerPool>,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let (server, worker_pool) = run(
            listener,
            connection_pool,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.application.jwt,
            configuration.redis_uri,
            configuration.s3,
            configuration.storage,
        )
        .await?;

        Ok(Self { port, server, worker_pool })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        let result = self.server.await;
        if let Some(pool) = self.worker_pool {
            pool.shutdown().await;
        }
        result
    }
}
pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}

pub struct ApplicationBaseUrl(pub String);
async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    base_url: String,
    hmac_secret: SecretString,
    jwt_settings: JwtSettings,
    redis_uri: SecretString,
    s3_settings: Option<S3Settings>,
    storage_config: crate::clients::StorageConfig,
) -> Result<(Server, Option<crate::jobs::WorkerPool>), anyhow::Error> {
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    let jwt_settings = web::Data::new(jwt_settings);

    let db_pool = web::Data::new(db_pool);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;

    // S3 client (use ref to keep s3_settings available for storage provider and job handler)
    let s3_client_opt = match &s3_settings {
        Some(s3_config) => Some(get_s3_client(s3_config.clone()).await?),
        None => None,
    };
    let s3_client = web::Data::new(s3_client_opt.clone());

    let app_metrics = web::Data::new(crate::metrics::AppMetrics::new());

    // ── Event Publisher ──
    let event_publisher: Arc<dyn EventPublisher> = Arc::new(LogEventPublisher);
    let event_publisher = web::Data::new(event_publisher);

    // ── Rate Limiter ──
    let redis_client = Arc::new(redis::Client::open(redis_uri.expose_secret().to_string())?);
    let rate_limiter = web::Data::new(RateLimiter::new(
        redis_client,
        RateLimitConfig::default(),
    ));

    // ── Storage Provider (best-effort) ──
    let storage_provider: Option<web::Data<Arc<dyn StorageProvider>>> =
        match create_storage_provider(&storage_config, s3_settings.as_ref()).await {
            Ok(provider) => {
                tracing::info!(
                    provider = provider.provider_name(),
                    "Storage provider initialized"
                );
                Some(web::Data::new(Arc::from(provider)))
            }
            Err(e) => {
                tracing::warn!("Failed to create storage provider: {}", e);
                None
            }
        };

    // ── Job System ──
    let mut worker_pool: Option<crate::jobs::WorkerPool> = None;
    let redis_url = redis_uri.expose_secret().to_string();
    let mut redis_streams_config = crate::jobs::RedisStreamsConfig::default();
    redis_streams_config.url = redis_url.clone();

    match crate::jobs::RedisStreamsQueue::with_pool(
        redis_streams_config,
        db_pool.get_ref().clone(),
    ) {
        Ok(queue) => {
            let queue = Arc::new(queue);

            // Initialize streams and consumer groups (best-effort)
            if let Err(e) = queue.initialize().await {
                tracing::warn!("Failed to initialize job queue streams: {}", e);
            }

            // Register job handlers
            let mut handlers = crate::jobs::JobHandlers::new();
            if let Some(ref s3) = s3_client_opt {
                match crate::jobs::DownloadJobHandler::new(
                    db_pool.get_ref().clone(),
                    Arc::new(s3.clone()),
                    &redis_url,
                ) {
                    Ok(handler) => {
                        handlers.register(handler);
                        tracing::info!("Registered DownloadJobHandler");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create DownloadJobHandler: {}", e);
                    }
                }
            }

            let handlers = Arc::new(handlers);

            // Start worker pool if any handlers are registered
            if !handlers.registered_types().is_empty() {
                let pool = crate::jobs::WorkerPool::start(
                    crate::jobs::WorkerPoolConfig::default(),
                    queue,
                    handlers,
                )
                .await;
                tracing::info!("Started job worker pool");
                worker_pool = Some(pool);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to create job queue: {}", e);
        }
    }

    let server = HttpServer::new(move || {
        let mut app = App::new()
            .wrap(NormalizePath::trim())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .wrap(TracingLogger::default())
            .service(
                web::scope("/api/v1")
                    // Public routes (no auth required)
                    .route("/auth", web::post().to(login))
                    .route("/register", web::post().to(register))
                    .route("/health_check", web::get().to(health_check))
                    .route("/health", web::get().to(detailed_health_check))
                    // Password reset (public)
                    .route("/auth/forgot-password", web::post().to(forgot_password))
                    .route("/auth/reset-password", web::post().to(reset_password))
                    // Email verification (public - token in body)
                    .route("/auth/verify-email", web::post().to(verify_email))
                    // Protected routes (auth required + rate limited)
                    .service(
                        web::scope("")
                            .wrap(from_fn(rate_limit_middleware))
                            .wrap(from_fn(reject_anonymous_users))
                            // User profile
                            .route("/me", web::get().to(get_current_user))
                            .route("/me", web::patch().to(update_profile))
                            .route("/me", web::delete().to(delete_account))
                            .route("/me/password", web::post().to(change_password))
                            // Email verification (requires auth for resend)
                            .route("/auth/resend-verification", web::post().to(resend_verification))
                            // Downloads
                            .route("/download_file", web::get().to(download))
                            .route("/downloads/{id}", web::get().to(get_download))
                            .route("/downloads", web::get().to(get_downloads))
                            // Download control
                            .route("/downloads/{id}/progress", web::get().to(get_progress))
                            .route("/downloads/{id}/pause", web::post().to(pause_download))
                            .route("/downloads/{id}/resume", web::post().to(resume_download))
                            .route("/downloads/{id}/retry", web::post().to(retry_download))
                            .route("/downloads/{id}/cancel", web::post().to(cancel_download))
                            // Download tags
                            .route("/downloads/{id}/tags", web::get().to(get_tags_for_download))
                            .route("/downloads/{id}/tags", web::post().to(add_download_tags))
                            .route("/downloads/{download_id}/tags/{tag_id}", web::delete().to(remove_download_tag))
                            // Folders
                            .route("/folders", web::post().to(create_folder))
                            .route("/folders", web::get().to(list_root_folders))
                            .route("/folders/{id}", web::get().to(get_folder))
                            .route("/folders/{id}", web::patch().to(update_folder))
                            .route("/folders/{id}", web::delete().to(delete_folder))
                            .route("/folders/{id}/subfolders", web::get().to(get_folder_subfolders))
                            .route("/folders/{id}/move", web::post().to(move_folder))
                            // Tags
                            .route("/tags", web::post().to(create_tag))
                            .route("/tags", web::get().to(list_tags))
                            .route("/tags/{id}", web::get().to(get_tag))
                            .route("/tags/{id}", web::patch().to(update_tag))
                            .route("/tags/{id}", web::delete().to(delete_tag))
                            // Bulk operations
                            .route("/downloads/bulk/move", web::post().to(bulk_move))
                            .route("/downloads/bulk/tag", web::post().to(bulk_tag))
                            .route("/downloads/bulk/untag", web::post().to(bulk_untag))
                            .route("/downloads/bulk/delete", web::post().to(bulk_delete))
                            // Search
                            .route("/search", web::get().to(search_handler))
                            // Usage
                            .route("/me/usage", web::get().to(get_usage)),
                    )
                    // Admin routes (require auth + admin)
                    .service(
                        web::scope("/admin")
                            .wrap(from_fn(require_admin))
                            .wrap(from_fn(reject_anonymous_users))
                            .route("/stats", web::get().to(get_admin_stats))
                            .route("/users", web::get().to(admin_list_users))
                            .route("/users/{id}", web::get().to(admin_get_user))
                            .route("/users/{id}", web::patch().to(admin_update_user))
                            .route("/users/{id}", web::delete().to(admin_delete_user))
                            .route("/downloads", web::get().to(admin_list_downloads))
                            .route("/audit-logs", web::get().to(list_audit_logs))
                            // Metrics endpoint (requires admin auth)
                            .route("/metrics", web::get().to(metrics_handler)),
                    ),
            )
            .app_data(db_pool.clone())
            .app_data(s3_client.clone())
            .app_data(base_url.clone())
            .app_data(jwt_settings.clone())
            .app_data(app_metrics.clone())
            .app_data(event_publisher.clone())
            .app_data(rate_limiter.clone())
            .app_data(web::JsonConfig::default().error_handler(error_handler))
            .app_data(web::PathConfig::default().error_handler(error_handler))
            .app_data(web::QueryConfig::default().error_handler(error_handler));

        if let Some(ref sp) = storage_provider {
            app = app.app_data(sp.clone());
        }

        app
    })
    .listen(listener)?
    .run();

    Ok((server, worker_pool))
}
