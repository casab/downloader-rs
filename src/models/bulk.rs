//! Bulk operation models for batch operations on downloads.

use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

/// Request to move multiple downloads to a folder.
#[derive(Debug, Deserialize)]
pub struct BulkMoveRequest {
    /// IDs of downloads to move.
    pub download_ids: Vec<Uuid>,
    /// Target folder ID. If None, moves to root (no folder).
    pub folder_id: Option<Uuid>,
}

/// Request to add tags to multiple downloads.
#[derive(Debug, Deserialize)]
pub struct BulkTagRequest {
    /// IDs of downloads to tag.
    pub download_ids: Vec<Uuid>,
    /// Tag IDs to add.
    pub tag_ids: Vec<Uuid>,
}

/// Request to remove tags from multiple downloads.
#[derive(Debug, Deserialize)]
pub struct BulkUntagRequest {
    /// IDs of downloads to untag.
    pub download_ids: Vec<Uuid>,
    /// Tag IDs to remove.
    pub tag_ids: Vec<Uuid>,
}

/// Request to delete multiple downloads.
#[derive(Debug, Deserialize)]
pub struct BulkDeleteRequest {
    /// IDs of downloads to delete.
    pub download_ids: Vec<Uuid>,
}

/// Response for bulk operations.
#[derive(Debug, Serialize)]
pub struct BulkOperationResponse {
    /// Number of items successfully processed.
    pub success_count: i64,
    /// Number of items that failed.
    pub failure_count: i64,
    /// IDs of items that were successfully processed.
    pub successful_ids: Vec<Uuid>,
    /// IDs of items that failed with error messages.
    pub failed_items: Vec<BulkOperationFailure>,
}

impl BulkOperationResponse {
    /// Create a response for a fully successful operation.
    #[must_use]
    pub fn all_success(ids: Vec<Uuid>) -> Self {
        Self {
            success_count: ids.len() as i64,
            failure_count: 0,
            successful_ids: ids,
            failed_items: Vec::new(),
        }
    }

    /// Create a response for a fully failed operation.
    #[must_use]
    pub fn all_failed(failures: Vec<BulkOperationFailure>) -> Self {
        Self {
            success_count: 0,
            failure_count: failures.len() as i64,
            successful_ids: Vec::new(),
            failed_items: failures,
        }
    }
}

/// Details about a failed bulk operation item.
#[derive(Debug, Serialize)]
pub struct BulkOperationFailure {
    pub id: Uuid,
    pub error: String,
}

impl BulkOperationFailure {
    /// Create a new failure record.
    #[must_use]
    pub fn new(id: Uuid, error: impl Into<String>) -> Self {
        Self {
            id,
            error: error.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bulk_response_all_success() {
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let response = BulkOperationResponse::all_success(ids.clone());

        assert_eq!(response.success_count, 2);
        assert_eq!(response.failure_count, 0);
        assert_eq!(response.successful_ids, ids);
        assert!(response.failed_items.is_empty());
    }

    #[test]
    fn test_bulk_response_all_failed() {
        let failures = vec![
            BulkOperationFailure::new(Uuid::new_v4(), "Not found"),
            BulkOperationFailure::new(Uuid::new_v4(), "Permission denied"),
        ];
        let response = BulkOperationResponse::all_failed(failures);

        assert_eq!(response.success_count, 0);
        assert_eq!(response.failure_count, 2);
        assert!(response.successful_ids.is_empty());
        assert_eq!(response.failed_items.len(), 2);
    }
}
