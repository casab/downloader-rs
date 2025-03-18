use crate::clients::get_s3_client;
use crate::configuration::{DatabaseSettings, S3Settings, Settings};
use crate::middlewares::reject_anonymous_users;
use crate::routes::{download, get_download, get_downloads, health_check, login, register};
use actix_web::cookie::Key;
use actix_web::middleware::from_fn;

use crate::utils::error_handler;
use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use secrecy::{ExposeSecret, SecretString};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
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
        let server = run(
            listener,
            connection_pool,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri,
            configuration.s3,
        )
        .await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
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
    redis_uri: SecretString,
    s3_settings: Option<S3Settings>,
) -> Result<Server, anyhow::Error> {
    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());

    let db_pool = web::Data::new(db_pool);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;
    let s3_client = web::Data::new(if let Some(s3_config) = s3_settings {
        Some(get_s3_client(s3_config).await?)
    } else {
        None
    });

    let server = HttpServer::new(move || {
        App::new()
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
                    // Protected routes (auth required)
                    .service(
                        web::scope("")
                            .wrap(from_fn(reject_anonymous_users))
                            .route("/download_file", web::get().to(download))
                            .route("/downloads/{id}", web::get().to(get_download))
                            .route("/downloads", web::get().to(get_downloads))
                            .route("/health_check", web::get().to(health_check)),
                    ),
            )
            .app_data(db_pool.clone())
            .app_data(s3_client.clone())
            .app_data(base_url.clone())
            .app_data(web::JsonConfig::default().error_handler(error_handler))
            .app_data(web::PathConfig::default().error_handler(error_handler))
            .app_data(web::QueryConfig::default().error_handler(error_handler))
    })
    .listen(listener)?
    .run();

    Ok(server)
}
