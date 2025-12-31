use crate::helpers::{TestUser, spawn_app};
use downloader::models::{Download, DownloadStatus};
use sqlx::PgPool;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn create_test_download(pool: &PgPool, test_user: &TestUser) -> Download {
    sqlx::query_as!(
        Download,
        r#"
        INSERT INTO downloads (id, url, status, user_id, created_at, updated_at)
        VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        RETURNING id, url, user_id, status as "status: DownloadStatus", file_path, created_at, updated_at, completed_at
        "#,
        uuid::Uuid::new_v4(),
        "https://example.com/test.zip",
        "PENDING",
        test_user.id
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create test download")
}

#[tokio::test]
async fn get_download_returns_200_for_existing_download() {
    // Arrange
    let app = spawn_app().await;
    let download = create_test_download(&app.db_pool, &app.test_user).await;

    // Act
    app.test_user.login(&app).await;

    let response = app
        .api_client
        .get(&format!(
            "{}/api/v1/downloads/{}",
            &app.address, download.id
        ))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 200);

    let returned_download: Download = response.json().await.expect("Failed to parse response");
    assert_eq!(returned_download.id, download.id);
    assert_eq!(returned_download.url, "https://example.com/test.zip");
    assert!(matches!(returned_download.status, DownloadStatus::Pending));
}

#[tokio::test]
async fn get_download_returns_404_for_non_existent_download() {
    // Arrange
    let app = spawn_app().await;
    let non_existent_id = uuid::Uuid::new_v4();

    // Act
    app.test_user.login(&app).await;

    let response = app
        .api_client
        .get(&format!(
            "{}/api/v1/downloads/{}",
            &app.address, non_existent_id
        ))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 404);
}

#[tokio::test]
async fn get_downloads_returns_200_and_list() {
    // Arrange
    let app = spawn_app().await;
    let download1 = create_test_download(&app.db_pool, &app.test_user).await;
    let download2 = create_test_download(&app.db_pool, &app.test_user).await;

    // Act
    app.test_user.login(&app).await;

    let response = app
        .api_client
        .get(&format!("{}/api/v1/downloads", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 200);

    let returned_downloads: Vec<Download> =
        response.json().await.expect("Failed to parse response");
    assert_eq!(returned_downloads.len(), 2);
    assert!(returned_downloads.iter().any(|d| d.id == download1.id));
    assert!(returned_downloads.iter().any(|d| d.id == download2.id));
}

#[tokio::test]
async fn download_file_returns_200_for_valid_url() {
    // Arrange
    let app = spawn_app().await;
    let mock_server = MockServer::start().await;
    let test_file_content = "test file content";

    Mock::given(method("GET"))
        .and(path("/test.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_string(test_file_content))
        .mount(&mock_server)
        .await;

    let test_url = format!("{}/test.zip", &mock_server.uri());

    // Act
    app.test_user.login(&app).await;

    let response = app
        .api_client
        .get(&format!(
            "{}/api/v1/download_file?url={}",
            &app.address, test_url
        ))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn download_file_returns_500_for_server_error() {
    // Arrange
    let app = spawn_app().await;
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/error.zip"))
        .respond_with(ResponseTemplate::new(500).insert_header("content-length", "52"))
        .mount(&mock_server)
        .await;

    let test_url = format!("{}/error.zip", &mock_server.uri());

    // Act
    app.test_user.login(&app).await;

    let response = app
        .api_client
        .get(&format!(
            "{}/api/v1/download_file?url={}",
            &app.address, test_url
        ))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 500);
}
