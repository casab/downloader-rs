//! S3-compatible storage provider.

use super::{StorageMetadata, StorageObject, StorageProvider};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use s3::Bucket;
use std::time::Duration;

/// S3-compatible storage provider (works with AWS S3, MinIO, etc.).
pub struct S3StorageProvider {
    bucket: Box<Bucket>,
}

impl S3StorageProvider {
    /// Create a new S3 storage provider from an existing bucket.
    #[must_use]
    pub fn new(bucket: Box<Bucket>) -> Self {
        Self { bucket }
    }

    /// Create from the existing S3Client type.
    #[must_use]
    pub fn from_client(client: crate::clients::S3Client) -> Self {
        Self { bucket: client }
    }
}

#[async_trait]
impl StorageProvider for S3StorageProvider {
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String> {
        self.bucket
            .put_object_with_content_type(key, data, content_type)
            .await
            .with_context(|| format!("Failed to upload to S3: {key}"))?;

        Ok(key.to_string())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let response = self
            .bucket
            .get_object(key)
            .await
            .with_context(|| format!("Failed to download from S3: {key}"))?;

        Ok(response.to_vec())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.bucket
            .delete_object(key)
            .await
            .with_context(|| format!("Failed to delete from S3: {key}"))?;

        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        match self.bucket.head_object(key).await {
            Ok(_) => Ok(true),
            Err(s3::error::S3Error::HttpFailWithBody(404, _)) => Ok(false),
            Err(e) => Err(e).with_context(|| format!("Failed to check existence in S3: {key}")),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    async fn get_url(&self, key: &str, expires_in: Duration) -> Result<String> {
        let url = self
            .bucket
            .presign_get(key, expires_in.as_secs() as u32, None)
            .await
            .with_context(|| format!("Failed to generate presigned URL: {key}"))?;

        Ok(url)
    }

    #[allow(clippy::cast_possible_wrap)]
    async fn list(&self, prefix: &str) -> Result<Vec<StorageObject>> {
        let results = self
            .bucket
            .list(prefix.to_string(), None)
            .await
            .with_context(|| format!("Failed to list S3 objects with prefix: {prefix}"))?;

        let mut objects = Vec::new();
        for result in results {
            for item in result.contents {
                objects.push(StorageObject {
                    key: item.key,
                    size: item.size as i64,
                    last_modified: item.last_modified.parse().unwrap_or_else(|_| Utc::now()),
                    content_type: None,
                });
            }
        }

        Ok(objects)
    }

    async fn get_metadata(&self, key: &str) -> Result<StorageMetadata> {
        let (head, _) = self
            .bucket
            .head_object(key)
            .await
            .with_context(|| format!("Failed to get S3 object metadata: {key}"))?;

        Ok(StorageMetadata {
            key: key.to_string(),
            size: head.content_length.unwrap_or(0),
            content_type: head.content_type,
            last_modified: head
                .last_modified
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(Utc::now),
            etag: head.e_tag,
        })
    }

    fn provider_name(&self) -> &'static str {
        "s3"
    }
}
