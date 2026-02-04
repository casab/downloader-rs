//! Tag model for labeling downloads.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

/// Database row representation of a tag.
#[derive(Debug, sqlx::FromRow)]
pub struct TagRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<TagRow> for Tag {
    fn from(row: TagRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            name: row.name,
            color: row.color,
            created_at: row.created_at,
        }
    }
}

/// Tag for labeling downloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    /// Hex color for UI display (e.g., "#FF5733").
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Tag {
    /// Validate the tag name.
    pub fn validate_name(name: &str) -> Result<(), TagValidationError> {
        if name.is_empty() {
            return Err(TagValidationError::EmptyName);
        }
        if name.len() > 50 {
            return Err(TagValidationError::NameTooLong);
        }
        if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ') {
            return Err(TagValidationError::InvalidCharacters);
        }
        Ok(())
    }

    /// Validate the color format.
    pub fn validate_color(color: &str) -> Result<(), TagValidationError> {
        if color.is_empty() {
            return Ok(()); // Empty color is allowed (will use default)
        }
        if !color.starts_with('#') {
            return Err(TagValidationError::InvalidColorFormat);
        }
        if color.len() != 7 {
            return Err(TagValidationError::InvalidColorFormat);
        }
        if !color[1..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(TagValidationError::InvalidColorFormat);
        }
        Ok(())
    }
}

/// Tag validation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagValidationError {
    EmptyName,
    NameTooLong,
    InvalidCharacters,
    InvalidColorFormat,
}

impl std::fmt::Display for TagValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "Tag name cannot be empty"),
            Self::NameTooLong => write!(f, "Tag name cannot exceed 50 characters"),
            Self::InvalidCharacters => write!(f, "Tag name contains invalid characters"),
            Self::InvalidColorFormat => write!(f, "Color must be a valid hex color (e.g., #FF5733)"),
        }
    }
}

impl std::error::Error for TagValidationError {}

/// Tag with usage count.
#[derive(Debug, Serialize)]
pub struct TagWithCount {
    #[serde(flatten)]
    pub tag: Tag,
    pub download_count: i64,
}

/// Request to create a new tag.
#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
}

/// Request to update a tag.
#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub color: Option<String>,
}

/// Request to add tags to a download.
#[derive(Debug, Deserialize)]
pub struct AddTagsRequest {
    pub tag_ids: Vec<Uuid>,
}

/// Download-tag association.
#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct DownloadTag {
    pub download_id: Uuid,
    pub tag_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(Tag::validate_name("work").is_ok());
        assert!(Tag::validate_name("Work Documents").is_ok());
        assert!(Tag::validate_name("high-priority").is_ok());
        assert!(Tag::validate_name("tag_123").is_ok());
    }

    #[test]
    fn test_validate_name_empty() {
        assert_eq!(Tag::validate_name("").unwrap_err(), TagValidationError::EmptyName);
    }

    #[test]
    fn test_validate_name_too_long() {
        let long_name = "a".repeat(51);
        assert_eq!(Tag::validate_name(&long_name).unwrap_err(), TagValidationError::NameTooLong);
    }

    #[test]
    fn test_validate_name_invalid_chars() {
        assert_eq!(Tag::validate_name("tag@name").unwrap_err(), TagValidationError::InvalidCharacters);
        assert_eq!(Tag::validate_name("tag#name").unwrap_err(), TagValidationError::InvalidCharacters);
    }

    #[test]
    fn test_validate_color_valid() {
        assert!(Tag::validate_color("#FF5733").is_ok());
        assert!(Tag::validate_color("#000000").is_ok());
        assert!(Tag::validate_color("#ffffff").is_ok());
        assert!(Tag::validate_color("").is_ok()); // Empty is allowed
    }

    #[test]
    fn test_validate_color_invalid() {
        assert_eq!(Tag::validate_color("FF5733").unwrap_err(), TagValidationError::InvalidColorFormat);
        assert_eq!(Tag::validate_color("#FFF").unwrap_err(), TagValidationError::InvalidColorFormat);
        assert_eq!(Tag::validate_color("#GGGGGG").unwrap_err(), TagValidationError::InvalidColorFormat);
    }
}
