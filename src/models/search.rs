//! Search models for full-text search functionality.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

use super::DownloadStatus;

/// Search query parameters.
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// Search query string.
    pub q: String,
    /// Scope search to a specific folder and its descendants.
    #[serde(default)]
    pub folder_id: Option<Uuid>,
    /// Filter by tag names.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Filter by download status.
    #[serde(default)]
    pub status: Option<Vec<DownloadStatus>>,
    /// Filter by MIME type (e.g., "application/pdf").
    #[serde(default)]
    pub content_type: Option<String>,
    /// Minimum file size in bytes.
    #[serde(default)]
    pub min_size: Option<i64>,
    /// Maximum file size in bytes.
    #[serde(default)]
    pub max_size: Option<i64>,
    /// Created after this date.
    #[serde(default)]
    pub created_after: Option<DateTime<Utc>>,
    /// Created before this date.
    #[serde(default)]
    pub created_before: Option<DateTime<Utc>>,
    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: i64,
    /// Items per page.
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

impl SearchQuery {
    /// Get the offset for pagination.
    #[must_use]
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.per_page
    }

    /// Get the limit for pagination.
    #[must_use]
    pub fn limit(&self) -> i64 {
        self.per_page.clamp(1, 100)
    }
}

/// Type of search result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchResultType {
    Download,
    Folder,
}

/// Individual search result.
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: Uuid,
    pub result_type: SearchResultType,
    /// Display title (filename or folder name).
    pub title: String,
    /// Highlighted snippet showing the match context.
    pub snippet: Option<String>,
    /// Relevance score (higher is more relevant).
    pub score: f32,
    /// Full entity data.
    pub data: serde_json::Value,
}

/// Search response with results and metadata.
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    /// Total number of matching results.
    pub total: i64,
    /// Time taken in milliseconds.
    pub took_ms: i64,
    /// Search suggestions for autocomplete.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
}

impl SearchResponse {
    /// Create an empty search response.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            results: Vec::new(),
            total: 0,
            took_ms: 0,
            suggestions: Vec::new(),
        }
    }
}

/// Search suggestion for autocomplete.
#[derive(Debug, Serialize)]
pub struct SearchSuggestion {
    pub text: String,
    pub highlight: String,
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_offset() {
        let mut query = SearchQuery {
            q: "test".to_string(),
            folder_id: None,
            tags: None,
            status: None,
            content_type: None,
            min_size: None,
            max_size: None,
            created_after: None,
            created_before: None,
            page: 1,
            per_page: 20,
        };

        assert_eq!(query.offset(), 0);

        query.page = 2;
        assert_eq!(query.offset(), 20);

        query.page = 3;
        query.per_page = 10;
        assert_eq!(query.offset(), 20);
    }

    #[test]
    fn test_search_query_limit() {
        let mut query = SearchQuery {
            q: "test".to_string(),
            folder_id: None,
            tags: None,
            status: None,
            content_type: None,
            min_size: None,
            max_size: None,
            created_after: None,
            created_before: None,
            page: 1,
            per_page: 20,
        };

        assert_eq!(query.limit(), 20);

        query.per_page = 150;
        assert_eq!(query.limit(), 100); // Clamped to max

        query.per_page = 0;
        assert_eq!(query.limit(), 1); // Clamped to min
    }

    #[test]
    fn test_search_response_empty() {
        let response = SearchResponse::empty();
        assert!(response.results.is_empty());
        assert_eq!(response.total, 0);
        assert_eq!(response.took_ms, 0);
    }
}
