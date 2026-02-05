//! Storage abstraction layer for multi-provider support.

mod local;
mod s3;

pub use local::LocalStorageProvider;
pub use s3::S3StorageProvider;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Trait for storage provider implementations.
#[async_trait]
pub trait StorageProvider: Send + Sync {
    /// Upload data to storage.
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String>;

    /// Download data from storage.
    async fn download(&self, key: &str) -> Result<Vec<u8>>;

    /// Delete an object from storage.
    async fn delete(&self, key: &str) -> Result<()>;

    /// Check if an object exists in storage.
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Get a presigned/temporary URL for an object.
    async fn get_url(&self, key: &str, expires_in: Duration) -> Result<String>;

    /// List objects with a given prefix.
    async fn list(&self, prefix: &str) -> Result<Vec<StorageObject>>;

    /// Get metadata for a specific object.
    async fn get_metadata(&self, key: &str) -> Result<StorageMetadata>;

    /// Get the provider name.
    fn provider_name(&self) -> &'static str;
}

/// Metadata about a stored object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageObject {
    pub key: String,
    pub size: i64,
    pub last_modified: DateTime<Utc>,
    pub content_type: Option<String>,
}

/// Detailed metadata for a specific object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetadata {
    pub key: String,
    pub size: i64,
    pub content_type: Option<String>,
    pub last_modified: DateTime<Utc>,
    pub etag: Option<String>,
}

/// Storage quota information for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageQuota {
    /// Bytes used by the user.
    pub used_bytes: i64,
    /// Maximum allowed bytes.
    pub quota_bytes: i64,
    /// Number of files stored.
    pub file_count: i64,
    /// Maximum allowed files (None = unlimited).
    pub max_file_count: Option<i64>,
}

impl StorageQuota {
    /// Check if the quota allows uploading additional bytes.
    #[must_use]
    pub fn can_upload(&self, additional_bytes: i64) -> bool {
        self.used_bytes + additional_bytes <= self.quota_bytes
    }

    /// Get usage percentage (0.0 - 100.0).
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn usage_percentage(&self) -> f64 {
        if self.quota_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f64 / self.quota_bytes as f64) * 100.0
    }

    /// Get remaining bytes.
    #[must_use]
    pub fn remaining_bytes(&self) -> i64 {
        (self.quota_bytes - self.used_bytes).max(0)
    }
}

/// Storage provider configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    /// Provider type: "s3", "local"
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Local storage settings.
    pub local: Option<LocalStorageConfig>,
    /// Default quota per user in bytes (default: 10 GB).
    #[serde(default = "default_quota_bytes")]
    pub quota_bytes: i64,
    /// Maximum files per user (None = unlimited).
    pub max_file_count: Option<i64>,
}

/// Local storage configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct LocalStorageConfig {
    /// Base path for local file storage.
    pub base_path: String,
}

fn default_provider() -> String {
    "local".to_string()
}

fn default_quota_bytes() -> i64 {
    10 * 1024 * 1024 * 1024 // 10 GB
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            local: Some(LocalStorageConfig {
                base_path: "/var/downloads".to_string(),
            }),
            quota_bytes: default_quota_bytes(),
            max_file_count: None,
        }
    }
}

/// Create a storage provider from configuration.
///
/// Selects the appropriate provider based on `StorageConfig::provider`:
/// - `"local"` -> `LocalStorageProvider`
/// - `"s3"` -> `S3StorageProvider` (requires S3 settings)
pub async fn create_storage_provider(
    config: &StorageConfig,
    s3_settings: Option<&crate::configuration::S3Settings>,
) -> anyhow::Result<Box<dyn StorageProvider>> {
    if config.provider.as_str() == "s3" {
        let s3_config = s3_settings
            .ok_or_else(|| anyhow::anyhow!("S3 settings required when provider is 's3'"))?;
        let bucket = crate::clients::get_s3_client(s3_config.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create S3 client: {e}"))?;
        Ok(Box::new(S3StorageProvider::new(bucket)))
    } else {
        let local_config = config.local.clone().unwrap_or(LocalStorageConfig {
            base_path: "/var/downloads".to_string(),
        });
        let provider = LocalStorageProvider::new(&local_config).await?;
        Ok(Box::new(provider))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_quota_can_upload() {
        let quota = StorageQuota {
            used_bytes: 500,
            quota_bytes: 1000,
            file_count: 5,
            max_file_count: None,
        };

        assert!(quota.can_upload(400));
        assert!(quota.can_upload(500));
        assert!(!quota.can_upload(501));
    }

    #[test]
    fn test_storage_quota_percentage() {
        let quota = StorageQuota {
            used_bytes: 250,
            quota_bytes: 1000,
            file_count: 2,
            max_file_count: None,
        };

        assert!((quota.usage_percentage() - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_storage_quota_remaining() {
        let quota = StorageQuota {
            used_bytes: 700,
            quota_bytes: 1000,
            file_count: 3,
            max_file_count: None,
        };

        assert_eq!(quota.remaining_bytes(), 300);
    }

    #[test]
    fn test_storage_quota_zero_quota() {
        let quota = StorageQuota {
            used_bytes: 0,
            quota_bytes: 0,
            file_count: 0,
            max_file_count: None,
        };

        assert!((quota.usage_percentage() - 0.0).abs() < f64::EPSILON);
        assert!(!quota.can_upload(1));
    }

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert_eq!(config.provider, "local");
        assert!(config.local.is_some());
        assert_eq!(config.quota_bytes, 10 * 1024 * 1024 * 1024);
        assert!(config.max_file_count.is_none());
    }

    #[tokio::test]
    async fn test_create_local_storage_provider() {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let config = StorageConfig {
            provider: "local".to_string(),
            local: Some(LocalStorageConfig {
                base_path: temp_dir.path().to_string_lossy().to_string(),
            }),
            quota_bytes: 1024,
            max_file_count: None,
        };

        let provider = create_storage_provider(&config, None)
            .await
            .expect("Failed to create local storage provider");
        assert_eq!(provider.provider_name(), "local");
    }

    #[tokio::test]
    async fn test_create_s3_provider_without_settings_fails() {
        let config = StorageConfig {
            provider: "s3".to_string(),
            local: None,
            quota_bytes: 1024,
            max_file_count: None,
        };

        let result = create_storage_provider(&config, None).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("S3 settings required"));
    }
}
