//! Local filesystem storage provider.

use super::{LocalStorageConfig, StorageMetadata, StorageObject, StorageProvider};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;

/// Local filesystem storage provider for development and backup.
pub struct LocalStorageProvider {
    base_path: PathBuf,
}

impl LocalStorageProvider {
    /// Create a new local storage provider.
    pub async fn new(config: &LocalStorageConfig) -> Result<Self> {
        let base_path = PathBuf::from(&config.base_path);

        // Ensure base directory exists
        fs::create_dir_all(&base_path).await.with_context(|| {
            format!(
                "Failed to create storage directory: {}",
                base_path.display()
            )
        })?;

        Ok(Self { base_path })
    }

    /// Get the full filesystem path for a storage key, with path traversal protection.
    fn key_to_path(&self, key: &str) -> Result<PathBuf> {
        // Reject keys with path traversal components
        for component in Path::new(key).components() {
            match component {
                std::path::Component::ParentDir => {
                    anyhow::bail!("Invalid storage key: path traversal ('..') not allowed");
                },
                std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                    anyhow::bail!("Invalid storage key: absolute paths not allowed");
                },
                _ => {},
            }
        }
        Ok(self.base_path.join(key))
    }

    /// Ensure parent directories exist for a key.
    async fn ensure_parent_dirs(&self, key: &str) -> Result<()> {
        let path = self.key_to_path(key)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.with_context(|| {
                format!("Failed to create parent directory: {}", parent.display())
            })?;
        }
        Ok(())
    }

    /// Convert system time to DateTime<Utc>.
    fn system_time_to_datetime(time: std::time::SystemTime) -> DateTime<Utc> {
        DateTime::from(time)
    }
}

#[async_trait]
impl StorageProvider for LocalStorageProvider {
    async fn upload(&self, key: &str, data: &[u8], _content_type: &str) -> Result<String> {
        self.ensure_parent_dirs(key).await?;
        let path = self.key_to_path(key)?;

        fs::write(&path, data)
            .await
            .with_context(|| format!("Failed to write file: {}", path.display()))?;

        Ok(key.to_string())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let path = self.key_to_path(key)?;

        fs::read(&path)
            .await
            .with_context(|| format!("Failed to read file: {}", path.display()))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let path = self.key_to_path(key)?;

        if path.exists() {
            fs::remove_file(&path)
                .await
                .with_context(|| format!("Failed to delete file: {}", path.display()))?;
        }

        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let path = self.key_to_path(key)?;
        Ok(path.exists())
    }

    async fn get_url(&self, key: &str, _expires_in: Duration) -> Result<String> {
        // For local storage, return the file path as the URL
        let path = self.key_to_path(key)?;
        Ok(format!("file://{}", path.display()))
    }

    #[allow(clippy::cast_possible_wrap)]
    async fn list(&self, prefix: &str) -> Result<Vec<StorageObject>> {
        let search_path = self.key_to_path(prefix)?;
        let search_dir = if search_path.is_dir() {
            search_path
        } else {
            search_path
                .parent()
                .map_or_else(|| self.base_path.clone(), Path::to_path_buf)
        };

        if !search_dir.exists() {
            return Ok(Vec::new());
        }

        let mut objects = Vec::new();
        let mut entries = fs::read_dir(&search_dir)
            .await
            .with_context(|| format!("Failed to read directory: {}", search_dir.display()))?;

        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                let entry_path = entry.path();
                let key = entry_path
                    .strip_prefix(&self.base_path)
                    .unwrap_or(&entry_path)
                    .to_string_lossy()
                    .to_string();

                if key.starts_with(prefix) {
                    let last_modified = metadata
                        .modified()
                        .map_or_else(|_| Utc::now(), Self::system_time_to_datetime);

                    objects.push(StorageObject {
                        key,
                        size: metadata.len() as i64,
                        last_modified,
                        content_type: None,
                    });
                }
            }
        }

        Ok(objects)
    }

    #[allow(clippy::cast_possible_wrap)]
    async fn get_metadata(&self, key: &str) -> Result<StorageMetadata> {
        let path = self.key_to_path(key)?;
        let metadata = fs::metadata(&path)
            .await
            .with_context(|| format!("Failed to get metadata: {}", path.display()))?;

        let last_modified = metadata
            .modified()
            .map_or_else(|_| Utc::now(), Self::system_time_to_datetime);

        Ok(StorageMetadata {
            key: key.to_string(),
            size: metadata.len() as i64,
            content_type: None,
            last_modified,
            etag: None,
        })
    }

    fn provider_name(&self) -> &'static str {
        "local"
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_possible_wrap)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_provider() -> (LocalStorageProvider, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config = LocalStorageConfig {
            base_path: temp_dir.path().to_string_lossy().to_string(),
        };
        let provider = LocalStorageProvider::new(&config)
            .await
            .expect("Failed to create provider");
        (provider, temp_dir)
    }

    #[tokio::test]
    async fn test_upload_and_download() {
        let (provider, _dir) = create_test_provider().await;

        let data = b"Hello, world!";
        let key = "test/file.txt";

        provider
            .upload(key, data, "text/plain")
            .await
            .expect("Upload failed");

        let downloaded = provider.download(key).await.expect("Download failed");
        assert_eq!(downloaded, data);
    }

    #[tokio::test]
    async fn test_exists() {
        let (provider, _dir) = create_test_provider().await;

        assert!(
            !provider
                .exists("nonexistent")
                .await
                .expect("Exists check failed")
        );

        provider
            .upload("exists.txt", b"data", "text/plain")
            .await
            .expect("Upload failed");

        assert!(
            provider
                .exists("exists.txt")
                .await
                .expect("Exists check failed")
        );
    }

    #[tokio::test]
    async fn test_delete() {
        let (provider, _dir) = create_test_provider().await;

        provider
            .upload("delete_me.txt", b"data", "text/plain")
            .await
            .expect("Upload failed");

        assert!(
            provider
                .exists("delete_me.txt")
                .await
                .expect("Exists failed")
        );

        provider
            .delete("delete_me.txt")
            .await
            .expect("Delete failed");

        assert!(
            !provider
                .exists("delete_me.txt")
                .await
                .expect("Exists failed")
        );
    }

    #[tokio::test]
    async fn test_get_metadata() {
        let (provider, _dir) = create_test_provider().await;
        let data = b"test data for metadata";

        provider
            .upload("meta.txt", data, "text/plain")
            .await
            .expect("Upload failed");

        let metadata = provider
            .get_metadata("meta.txt")
            .await
            .expect("Metadata failed");
        assert_eq!(metadata.key, "meta.txt");
        assert_eq!(metadata.size, data.len() as i64);
    }

    #[tokio::test]
    async fn test_list() {
        let (provider, _dir) = create_test_provider().await;

        provider
            .upload("prefix/a.txt", b"a", "text/plain")
            .await
            .expect("Upload failed");
        provider
            .upload("prefix/b.txt", b"bb", "text/plain")
            .await
            .expect("Upload failed");
        provider
            .upload("other/c.txt", b"ccc", "text/plain")
            .await
            .expect("Upload failed");

        let objects = provider.list("prefix/").await.expect("List failed");
        assert_eq!(objects.len(), 2);
    }

    #[tokio::test]
    async fn test_provider_name() {
        let (provider, _dir) = create_test_provider().await;
        assert_eq!(provider.provider_name(), "local");
    }

    #[tokio::test]
    async fn test_path_traversal_rejected() {
        let (provider, _dir) = create_test_provider().await;

        let result = provider.upload("../escape.txt", b"bad", "text/plain").await;
        assert!(result.is_err());

        let result = provider.download("../../etc/passwd").await;
        assert!(result.is_err());

        let result = provider.exists("/etc/passwd").await;
        assert!(result.is_err());
    }
}
