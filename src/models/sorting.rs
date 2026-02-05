//! Sorting models for list endpoints.

use serde::Deserialize;

/// Sort order direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// Ascending order (A-Z, oldest first).
    #[default]
    Asc,
    /// Descending order (Z-A, newest first).
    Desc,
}

impl SortOrder {
    /// Get the SQL keyword for this sort order.
    #[must_use]
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Asc => write!(f, "asc"),
            Self::Desc => write!(f, "desc"),
        }
    }
}

/// Query parameters for sorting.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SortParams {
    /// Field to sort by.
    pub sort_by: Option<String>,

    /// Sort order (asc or desc).
    #[serde(default)]
    pub sort_order: SortOrder,
}

impl SortParams {
    /// Create new sort params.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the sort field.
    #[must_use]
    pub fn sort_by(mut self, field: impl Into<String>) -> Self {
        self.sort_by = Some(field.into());
        self
    }

    /// Set the sort order.
    #[must_use]
    pub fn order(mut self, order: SortOrder) -> Self {
        self.sort_order = order;
        self
    }

    /// Validate the sort field against a whitelist.
    /// Returns the validated field name or None if invalid.
    #[must_use]
    pub fn validate_field<'a>(&self, allowed_fields: &[&'a str]) -> Option<&'a str> {
        self.sort_by.as_ref().and_then(|field| {
            allowed_fields
                .iter()
                .find(|&&f| f.eq_ignore_ascii_case(field))
                .copied()
        })
    }

    /// Get the SQL ORDER BY clause.
    /// Returns None if no valid sort field is set.
    #[must_use]
    pub fn to_order_by(&self, allowed_fields: &[&str]) -> Option<String> {
        self.validate_field(allowed_fields)
            .map(|field| format!("{} {}", field, self.sort_order.as_sql()))
    }

    /// Get the SQL ORDER BY clause with a default.
    #[must_use]
    pub fn to_order_by_or_default(
        &self,
        allowed_fields: &[&str],
        default_field: &str,
        default_order: SortOrder,
    ) -> String {
        self.to_order_by(allowed_fields)
            .unwrap_or_else(|| format!("{} {}", default_field, default_order.as_sql()))
    }
}

/// Allowed sort fields for downloads.
pub const DOWNLOAD_SORT_FIELDS: &[&str] =
    &["created_at", "updated_at", "completed_at", "status", "url"];

/// Default sort field for downloads.
pub const DOWNLOAD_DEFAULT_SORT: &str = "created_at";

/// Default sort order for downloads.
pub const DOWNLOAD_DEFAULT_ORDER: SortOrder = SortOrder::Desc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_order_sql() {
        assert_eq!(SortOrder::Asc.as_sql(), "ASC");
        assert_eq!(SortOrder::Desc.as_sql(), "DESC");
    }

    #[test]
    fn test_validate_field() {
        let params = SortParams {
            sort_by: Some("created_at".to_string()),
            sort_order: SortOrder::Desc,
        };

        assert_eq!(
            params.validate_field(DOWNLOAD_SORT_FIELDS),
            Some("created_at")
        );
    }

    #[test]
    fn test_validate_field_case_insensitive() {
        let params = SortParams {
            sort_by: Some("CREATED_AT".to_string()),
            sort_order: SortOrder::Desc,
        };

        assert_eq!(
            params.validate_field(DOWNLOAD_SORT_FIELDS),
            Some("created_at")
        );
    }

    #[test]
    fn test_validate_invalid_field() {
        let params = SortParams {
            sort_by: Some("invalid_field".to_string()),
            sort_order: SortOrder::Desc,
        };

        assert_eq!(params.validate_field(DOWNLOAD_SORT_FIELDS), None);
    }

    #[test]
    fn test_to_order_by() {
        let params = SortParams {
            sort_by: Some("created_at".to_string()),
            sort_order: SortOrder::Desc,
        };

        assert_eq!(
            params.to_order_by(DOWNLOAD_SORT_FIELDS),
            Some("created_at DESC".to_string())
        );
    }

    #[test]
    fn test_to_order_by_or_default() {
        let params = SortParams::default();

        assert_eq!(
            params.to_order_by_or_default(
                DOWNLOAD_SORT_FIELDS,
                DOWNLOAD_DEFAULT_SORT,
                DOWNLOAD_DEFAULT_ORDER
            ),
            "created_at DESC"
        );
    }
}
