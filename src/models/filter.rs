//! Filtering models for list endpoints.

use crate::models::DownloadStatus;
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Filter parameters for downloads.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DownloadFilter {
    /// Filter by download status.
    pub status: Option<DownloadStatus>,

    /// Filter by URL containing this string.
    pub url_contains: Option<String>,

    /// Filter by downloads created after this date.
    pub created_after: Option<DateTime<Utc>>,

    /// Filter by downloads created before this date.
    pub created_before: Option<DateTime<Utc>>,

    /// Filter by downloads completed after this date.
    pub completed_after: Option<DateTime<Utc>>,

    /// Filter by downloads completed before this date.
    pub completed_before: Option<DateTime<Utc>>,
}

impl DownloadFilter {
    /// Check if any filters are active.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.url_contains.is_none()
            && self.created_after.is_none()
            && self.created_before.is_none()
            && self.completed_after.is_none()
            && self.completed_before.is_none()
    }

    /// Build a WHERE clause fragment for the filter.
    /// Returns (clause, params) where clause is the SQL and params are the bind values.
    #[must_use]
    pub fn to_where_clauses(&self) -> Vec<FilterClause> {
        let mut clauses = Vec::new();

        if let Some(status) = self.status {
            clauses.push(FilterClause::Status(status));
        }

        if let Some(ref url_contains) = self.url_contains {
            clauses.push(FilterClause::UrlContains(url_contains.clone()));
        }

        if let Some(created_after) = self.created_after {
            clauses.push(FilterClause::CreatedAfter(created_after));
        }

        if let Some(created_before) = self.created_before {
            clauses.push(FilterClause::CreatedBefore(created_before));
        }

        if let Some(completed_after) = self.completed_after {
            clauses.push(FilterClause::CompletedAfter(completed_after));
        }

        if let Some(completed_before) = self.completed_before {
            clauses.push(FilterClause::CompletedBefore(completed_before));
        }

        clauses
    }
}

/// Individual filter clause types for building dynamic queries.
#[derive(Debug, Clone)]
pub enum FilterClause {
    /// Filter by status.
    Status(DownloadStatus),
    /// Filter by URL containing string.
    UrlContains(String),
    /// Filter by created after date.
    CreatedAfter(DateTime<Utc>),
    /// Filter by created before date.
    CreatedBefore(DateTime<Utc>),
    /// Filter by completed after date.
    CompletedAfter(DateTime<Utc>),
    /// Filter by completed before date.
    CompletedBefore(DateTime<Utc>),
}

impl FilterClause {
    /// Get the SQL column name for this clause.
    #[must_use]
    pub fn column(&self) -> &'static str {
        match self {
            Self::Status(_) => "status",
            Self::UrlContains(_) => "url",
            Self::CreatedAfter(_) | Self::CreatedBefore(_) => "created_at",
            Self::CompletedAfter(_) | Self::CompletedBefore(_) => "completed_at",
        }
    }

    /// Get the SQL operator for this clause.
    #[must_use]
    pub fn operator(&self) -> &'static str {
        match self {
            Self::Status(_) => "=",
            Self::UrlContains(_) => "ILIKE",
            Self::CreatedAfter(_) | Self::CompletedAfter(_) => ">=",
            Self::CreatedBefore(_) | Self::CompletedBefore(_) => "<=",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_filter() {
        let filter = DownloadFilter::default();
        assert!(filter.is_empty());
        assert!(filter.to_where_clauses().is_empty());
    }

    #[test]
    fn test_filter_with_status() {
        let filter = DownloadFilter {
            status: Some(DownloadStatus::Completed),
            ..Default::default()
        };
        assert!(!filter.is_empty());

        let clauses = filter.to_where_clauses();
        assert_eq!(clauses.len(), 1);
    }

    #[test]
    fn test_filter_with_multiple_clauses() {
        let filter = DownloadFilter {
            status: Some(DownloadStatus::Pending),
            url_contains: Some("example.com".to_string()),
            created_after: Some(Utc::now()),
            ..Default::default()
        };

        let clauses = filter.to_where_clauses();
        assert_eq!(clauses.len(), 3);
    }
}
