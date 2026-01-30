//! Pagination models for list endpoints.

use serde::{Deserialize, Serialize};

/// Default number of items per page.
pub const DEFAULT_PER_PAGE: u32 = 20;

/// Maximum number of items per page.
pub const MAX_PER_PAGE: u32 = 100;

/// Query parameters for pagination.
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-indexed). Defaults to 1.
    #[serde(default = "default_page")]
    pub page: u32,

    /// Number of items per page. Defaults to 20, max 100.
    #[serde(default = "default_per_page")]
    pub per_page: u32,

    /// Cursor for cursor-based pagination (alternative to page).
    pub cursor: Option<String>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: default_page(),
            per_page: default_per_page(),
            cursor: None,
        }
    }
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    DEFAULT_PER_PAGE
}

impl PaginationParams {
    /// Create new pagination params with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the page number.
    #[must_use]
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// Set items per page.
    #[must_use]
    pub fn per_page(mut self, per_page: u32) -> Self {
        self.per_page = per_page;
        self
    }

    /// Normalize the pagination parameters (ensure valid ranges).
    #[must_use]
    pub fn normalize(mut self) -> Self {
        // Ensure page is at least 1
        if self.page == 0 {
            self.page = 1;
        }

        // Ensure per_page is within bounds
        if self.per_page == 0 {
            self.per_page = DEFAULT_PER_PAGE;
        } else if self.per_page > MAX_PER_PAGE {
            self.per_page = MAX_PER_PAGE;
        }

        self
    }

    /// Calculate the offset for SQL queries.
    #[must_use]
    pub fn offset(&self) -> i64 {
        ((self.page.saturating_sub(1)) * self.per_page) as i64
    }

    /// Get the limit for SQL queries.
    #[must_use]
    pub fn limit(&self) -> i64 {
        self.per_page as i64
    }
}

/// Metadata about pagination in a response.
#[derive(Debug, Clone, Serialize)]
pub struct PaginationMeta {
    /// Current page number.
    pub page: u32,

    /// Number of items per page.
    pub per_page: u32,

    /// Total number of items across all pages.
    pub total: i64,

    /// Total number of pages.
    pub total_pages: u32,

    /// Whether there is a next page.
    pub has_next: bool,

    /// Whether there is a previous page.
    pub has_prev: bool,

    /// Cursor for the next page (for cursor-based pagination).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,

    /// Cursor for the previous page (for cursor-based pagination).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,
}

impl PaginationMeta {
    /// Create pagination metadata from params and total count.
    #[must_use]
    pub fn new(params: &PaginationParams, total: i64) -> Self {
        let total_pages = if params.per_page > 0 {
            ((total as f64) / (params.per_page as f64)).ceil() as u32
        } else {
            0
        };

        Self {
            page: params.page,
            per_page: params.per_page,
            total,
            total_pages,
            has_next: params.page < total_pages,
            has_prev: params.page > 1,
            next_cursor: None,
            prev_cursor: None,
        }
    }

    /// Set the next cursor.
    #[must_use]
    pub fn with_next_cursor(mut self, cursor: String) -> Self {
        self.next_cursor = Some(cursor);
        self
    }

    /// Set the previous cursor.
    #[must_use]
    pub fn with_prev_cursor(mut self, cursor: String) -> Self {
        self.prev_cursor = Some(cursor);
        self
    }
}

/// A paginated response wrapper.
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    /// The data items for this page.
    pub data: Vec<T>,

    /// Pagination metadata.
    pub pagination: PaginationMeta,
}

impl<T> PaginatedResponse<T> {
    /// Create a new paginated response.
    #[must_use]
    pub fn new(data: Vec<T>, params: &PaginationParams, total: i64) -> Self {
        Self {
            data,
            pagination: PaginationMeta::new(params, total),
        }
    }

    /// Create a paginated response with cursor information.
    #[must_use]
    pub fn with_cursors(
        data: Vec<T>,
        params: &PaginationParams,
        total: i64,
        next_cursor: Option<String>,
        prev_cursor: Option<String>,
    ) -> Self {
        let mut meta = PaginationMeta::new(params, total);
        meta.next_cursor = next_cursor;
        meta.prev_cursor = prev_cursor;

        Self {
            data,
            pagination: meta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params_defaults() {
        let params = PaginationParams::new();
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, DEFAULT_PER_PAGE);
    }

    #[test]
    fn test_pagination_params_normalize() {
        let params = PaginationParams {
            page: 0,
            per_page: 200,
            cursor: None,
        }
        .normalize();

        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, MAX_PER_PAGE);
    }

    #[test]
    fn test_pagination_offset() {
        let params = PaginationParams {
            page: 3,
            per_page: 20,
            cursor: None,
        };
        assert_eq!(params.offset(), 40);
        assert_eq!(params.limit(), 20);
    }

    #[test]
    fn test_pagination_meta() {
        let params = PaginationParams {
            page: 2,
            per_page: 10,
            cursor: None,
        };
        let meta = PaginationMeta::new(&params, 35);

        assert_eq!(meta.page, 2);
        assert_eq!(meta.per_page, 10);
        assert_eq!(meta.total, 35);
        assert_eq!(meta.total_pages, 4);
        assert!(meta.has_next);
        assert!(meta.has_prev);
    }

    #[test]
    fn test_pagination_first_page() {
        let params = PaginationParams {
            page: 1,
            per_page: 10,
            cursor: None,
        };
        let meta = PaginationMeta::new(&params, 25);

        assert!(!meta.has_prev);
        assert!(meta.has_next);
    }

    #[test]
    fn test_pagination_last_page() {
        let params = PaginationParams {
            page: 3,
            per_page: 10,
            cursor: None,
        };
        let meta = PaginationMeta::new(&params, 25);

        assert!(meta.has_prev);
        assert!(!meta.has_next);
    }
}
