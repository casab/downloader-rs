//! Unified query parameters combining pagination, filtering, and sorting.

use crate::models::{filter::DownloadFilter, pagination::PaginationParams, sorting::SortParams};
use serde::Deserialize;

/// Combined query parameters for download list endpoints.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DownloadQueryParams {
    // Pagination
    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: u32,

    /// Number of items per page.
    #[serde(default = "default_per_page")]
    pub per_page: u32,

    /// Cursor for cursor-based pagination.
    pub cursor: Option<String>,

    // Filtering
    /// Filter by download status.
    pub status: Option<String>,

    /// Filter by URL containing this string.
    pub url_contains: Option<String>,

    /// Filter by downloads created after this date (ISO 8601).
    pub created_after: Option<String>,

    /// Filter by downloads created before this date (ISO 8601).
    pub created_before: Option<String>,

    // Sorting
    /// Field to sort by.
    pub sort_by: Option<String>,

    /// Sort order (asc or desc).
    pub sort_order: Option<String>,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

impl DownloadQueryParams {
    /// Extract pagination parameters.
    #[must_use]
    pub fn pagination(&self) -> PaginationParams {
        PaginationParams {
            page: self.page,
            per_page: self.per_page,
            cursor: self.cursor.clone(),
        }
        .normalize()
    }

    /// Extract filter parameters.
    #[must_use]
    pub fn filter(&self) -> DownloadFilter {
        use crate::models::DownloadStatus;
        use chrono::DateTime;

        let status = self
            .status
            .as_ref()
            .and_then(|s| match s.to_uppercase().as_str() {
                "PENDING" => Some(DownloadStatus::Pending),
                "IN_PROGRESS" => Some(DownloadStatus::InProgress),
                "COMPLETED" => Some(DownloadStatus::Completed),
                "FAILED" => Some(DownloadStatus::Failed),
                _ => None,
            });

        let created_after = self
            .created_after
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let created_before = self
            .created_before
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        DownloadFilter {
            status,
            url_contains: self.url_contains.clone(),
            created_after,
            created_before,
            completed_after: None,
            completed_before: None,
        }
    }

    /// Extract sorting parameters.
    #[must_use]
    pub fn sorting(&self) -> SortParams {
        use crate::models::sorting::SortOrder;

        let sort_order = self
            .sort_order
            .as_ref()
            .map(|s| {
                if s.eq_ignore_ascii_case("desc") {
                    SortOrder::Desc
                } else {
                    SortOrder::Asc
                }
            })
            .unwrap_or_default();

        SortParams {
            sort_by: self.sort_by.clone(),
            sort_order,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pagination() {
        let params = DownloadQueryParams {
            page: 2,
            per_page: 50,
            ..Default::default()
        };

        let pagination = params.pagination();
        assert_eq!(pagination.page, 2);
        assert_eq!(pagination.per_page, 50);
    }

    #[test]
    fn test_extract_filter() {
        let params = DownloadQueryParams {
            status: Some("completed".to_string()),
            url_contains: Some("example".to_string()),
            ..Default::default()
        };

        let filter = params.filter();
        assert!(filter.status.is_some());
        assert_eq!(filter.url_contains, Some("example".to_string()));
    }

    #[test]
    fn test_extract_sorting() {
        let params = DownloadQueryParams {
            sort_by: Some("created_at".to_string()),
            sort_order: Some("desc".to_string()),
            ..Default::default()
        };

        let sorting = params.sorting();
        assert_eq!(sorting.sort_by, Some("created_at".to_string()));
    }
}
