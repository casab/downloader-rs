use crate::clients::{get_s3_client, S3Client};
use crate::configuration::{DatabaseSettings, Settings};
use crate::routes::{download, get_download, get_downloads, health_check};
use crate::utils::error_handler;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
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

        let s3_client = if let Some(s3_config) = configuration.s3 {
            Some(get_s3_client(s3_config).await?)
        } else {
            None
        };

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
            s3_client,
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
    s3_client: Option<S3Client>,
) -> Result<Server, anyhow::Error> {
    let db_pool = web::Data::new(db_pool);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let s3_client = web::Data::new(s3_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .service(
                web::scope("/api/v1")
                    .route("/download_file", web::get().to(download))
                    .route("/downloads/{id}", web::get().to(get_download))
                    .route("/downloads", web::get().to(get_downloads))
                    .route("/health_check", web::get().to(health_check)),
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
