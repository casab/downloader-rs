//! Test fixtures and factory functions for generating test data.
//!
//! This module provides builders and factories for creating test entities
//! with sensible defaults that can be customized as needed.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

// =============================================================================
// Download Fixtures
// =============================================================================

/// Factory for creating test downloads with various states.
pub struct DownloadFactory;

impl DownloadFactory {
    /// Create a new download builder with default values.
    pub fn new() -> DownloadBuilder {
        DownloadBuilder::default()
    }

    /// Create a pending download builder.
    pub fn pending() -> DownloadBuilder {
        DownloadBuilder::default().status("PENDING")
    }

    /// Create an in-progress download builder.
    pub fn in_progress() -> DownloadBuilder {
        DownloadBuilder::default()
            .status("IN_PROGRESS")
            .bytes_downloaded(1024)
            .total_bytes(10240)
    }

    /// Create a completed download builder.
    pub fn completed() -> DownloadBuilder {
        let now = Utc::now();
        DownloadBuilder::default()
            .status("COMPLETED")
            .bytes_downloaded(10240)
            .total_bytes(10240)
            .file_path("/downloads/test-file.zip")
            .completed_at(now)
    }

    /// Create a failed download builder.
    pub fn failed() -> DownloadBuilder {
        DownloadBuilder::default()
            .status("FAILED")
            .error_message("Connection timeout")
            .retry_count(3)
    }
}

/// Builder for creating test downloads with customizable fields.
#[derive(Debug, Clone)]
pub struct DownloadBuilder {
    pub id: Uuid,
    pub url: String,
    pub status: String,
    pub file_path: Option<String>,
    pub user_id: Option<Uuid>,
    pub bytes_downloaded: i64,
    pub total_bytes: Option<i64>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Default for DownloadBuilder {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            url: format!("https://example.com/files/{}.zip", Uuid::new_v4()),
            status: "PENDING".to_string(),
            file_path: None,
            user_id: None,
            bytes_downloaded: 0,
            total_bytes: None,
            error_message: None,
            retry_count: 0,
            completed_at: None,
        }
    }
}

impl DownloadBuilder {
    /// Set the download ID.
    pub fn id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    /// Set the download URL.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// Set the download status.
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    /// Set the file path.
    pub fn file_path(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Set the user ID (owner of the download).
    pub fn user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set bytes downloaded.
    pub fn bytes_downloaded(mut self, bytes: i64) -> Self {
        self.bytes_downloaded = bytes;
        self
    }

    /// Set total bytes.
    pub fn total_bytes(mut self, bytes: i64) -> Self {
        self.total_bytes = Some(bytes);
        self
    }

    /// Set error message.
    pub fn error_message(mut self, msg: impl Into<String>) -> Self {
        self.error_message = Some(msg.into());
        self
    }

    /// Set retry count.
    pub fn retry_count(mut self, count: i32) -> Self {
        self.retry_count = count;
        self
    }

    /// Set completed at timestamp.
    pub fn completed_at(mut self, at: DateTime<Utc>) -> Self {
        self.completed_at = Some(at);
        self
    }

    /// Build and store the download in the database.
    ///
    /// # Panics
    /// Panics if user_id is not set or if database insert fails.
    pub async fn build(self, pool: &PgPool) -> TestDownload {
        let user_id = self.user_id.expect("user_id must be set before building");

        sqlx::query(
            r#"
            INSERT INTO downloads (id, url, status, file_path, user_id, completed_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(self.id)
        .bind(&self.url)
        .bind(&self.status)
        .bind(&self.file_path)
        .bind(user_id)
        .bind(self.completed_at)
        .execute(pool)
        .await
        .expect("Failed to insert test download");

        TestDownload {
            id: self.id,
            url: self.url,
            status: self.status,
            file_path: self.file_path,
            user_id,
        }
    }
}

/// A test download that has been stored in the database.
#[derive(Debug, Clone)]
pub struct TestDownload {
    pub id: Uuid,
    pub url: String,
    pub status: String,
    pub file_path: Option<String>,
    pub user_id: Uuid,
}

// =============================================================================
// User Fixtures
// =============================================================================

/// Factory for creating test users with various configurations.
pub struct UserFactory;

impl UserFactory {
    /// Create a regular user builder.
    pub fn regular() -> UserBuilder {
        UserBuilder::default()
    }

    /// Create an admin user builder.
    pub fn admin() -> UserBuilder {
        UserBuilder::default().is_admin(true)
    }

    /// Create a verified user builder.
    pub fn verified() -> UserBuilder {
        UserBuilder::default().email_verified(true)
    }

    /// Create an unverified user builder.
    pub fn unverified() -> UserBuilder {
        UserBuilder::default().email_verified(false)
    }
}

/// Builder for creating test users.
#[derive(Debug, Clone)]
pub struct UserBuilder {
    pub id: Uuid,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
    pub is_admin: bool,
    pub email_verified: bool,
}

impl Default for UserBuilder {
    fn default() -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            email: format!("test-{}@example.com", id),
            password: "test-password-123".to_string(),
            display_name: None,
            is_admin: false,
            email_verified: false,
        }
    }
}

impl UserBuilder {
    /// Set the user ID.
    pub fn id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    /// Set the email.
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = email.into();
        self
    }

    /// Set the password.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = password.into();
        self
    }

    /// Set the display name.
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Set admin status.
    pub fn is_admin(mut self, is_admin: bool) -> Self {
        self.is_admin = is_admin;
        self
    }

    /// Set email verified status.
    pub fn email_verified(mut self, verified: bool) -> Self {
        self.email_verified = verified;
        self
    }

    // Note: build() would need the password hashing logic from helpers.rs
    // For now, users should use TestUser from helpers.rs for full user creation
}

// =============================================================================
// Request Body Fixtures
// =============================================================================

/// Common request body fixtures for testing.
pub mod requests {
    use serde_json::{Value, json};

    /// Create a login request body.
    pub fn login(email: &str, password: &str) -> Value {
        json!({
            "email": email,
            "password": password
        })
    }

    /// Create a registration request body.
    pub fn register(email: &str, password: &str) -> Value {
        json!({
            "email": email,
            "password": password
        })
    }

    /// Create a download request query.
    pub fn download_url(url: &str) -> String {
        format!("url={}", urlencoding::encode(url))
    }
}

// =============================================================================
// URL Fixtures
// =============================================================================

/// Common test URLs.
pub mod urls {
    /// A valid file URL for testing downloads.
    pub const VALID_FILE: &str = "https://example.com/test-file.zip";

    /// A URL that returns a 404.
    pub const NOT_FOUND: &str = "https://example.com/not-found";

    /// A URL that times out.
    pub const TIMEOUT: &str = "https://example.com/timeout";

    /// A large file URL for testing progress tracking.
    pub const LARGE_FILE: &str = "https://example.com/large-file.bin";
}

// =============================================================================
// Helper Traits
// =============================================================================

/// Extension trait for building multiple fixtures.
pub trait FixtureBatch {
    type Item;

    /// Create multiple items with the given builder.
    fn batch(count: usize) -> Vec<Self::Item>;
}

impl FixtureBatch for DownloadBuilder {
    type Item = DownloadBuilder;

    fn batch(count: usize) -> Vec<Self::Item> {
        (0..count).map(|_| DownloadFactory::new()).collect()
    }
}
