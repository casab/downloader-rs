//! Folder model for hierarchical download organization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

/// Database row representation of a folder.
#[derive(Debug, sqlx::FromRow)]
pub struct FolderRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<FolderRow> for Folder {
    fn from(row: FolderRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            parent_id: row.parent_id,
            name: row.name,
            path: row.path,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Folder for organizing downloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    /// Materialized path for hierarchical queries (e.g., "/parent/child/grandchild").
    pub path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Folder {
    /// Get the depth of this folder in the hierarchy (root = 1).
    #[must_use]
    pub fn depth(&self) -> usize {
        self.path.matches('/').count()
    }

    /// Check if this folder is a root folder (no parent).
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Check if this folder is an ancestor of another folder based on path.
    #[must_use]
    pub fn is_ancestor_of(&self, other: &Folder) -> bool {
        other.path.starts_with(&self.path) && other.path.len() > self.path.len()
    }

    /// Get the parent path from this folder's path.
    #[must_use]
    pub fn parent_path(&self) -> Option<String> {
        let parts: Vec<&str> = self.path.trim_matches('/').split('/').collect();
        if parts.len() > 1 {
            Some(format!("/{}", parts[..parts.len() - 1].join("/")))
        } else {
            None
        }
    }
}

/// Folder with its contents count.
#[derive(Debug, Serialize)]
pub struct FolderWithCounts {
    #[serde(flatten)]
    pub folder: Folder,
    pub subfolder_count: i64,
    pub download_count: i64,
}

/// Folder tree node for hierarchical display.
#[derive(Debug, Serialize)]
pub struct FolderTreeNode {
    #[serde(flatten)]
    pub folder: Folder,
    pub children: Vec<FolderTreeNode>,
    pub download_count: i64,
}

/// Request to create a new folder.
#[derive(Debug, Deserialize)]
pub struct CreateFolderRequest {
    pub name: String,
    pub parent_id: Option<Uuid>,
}

/// Request to update a folder.
#[derive(Debug, Deserialize)]
pub struct UpdateFolderRequest {
    pub name: Option<String>,
}

/// Request to move a folder.
#[derive(Debug, Deserialize)]
pub struct MoveFolderRequest {
    /// New parent folder ID. If None, moves to root.
    pub parent_id: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_folder(path: &str, parent_id: Option<Uuid>) -> Folder {
        Folder {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            parent_id,
            name: path.split('/').last().unwrap_or("root").to_string(),
            path: path.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_folder_depth() {
        let root = create_test_folder("/documents", None);
        assert_eq!(root.depth(), 1);

        let child = create_test_folder("/documents/work", Some(Uuid::new_v4()));
        assert_eq!(child.depth(), 2);

        let grandchild = create_test_folder("/documents/work/projects", Some(Uuid::new_v4()));
        assert_eq!(grandchild.depth(), 3);
    }

    #[test]
    fn test_folder_is_root() {
        let root = create_test_folder("/documents", None);
        assert!(root.is_root());

        let child = create_test_folder("/documents/work", Some(Uuid::new_v4()));
        assert!(!child.is_root());
    }

    #[test]
    fn test_folder_is_ancestor_of() {
        let parent = create_test_folder("/documents", None);
        let child = create_test_folder("/documents/work", Some(parent.id));
        let grandchild = create_test_folder("/documents/work/projects", Some(child.id));

        assert!(parent.is_ancestor_of(&child));
        assert!(parent.is_ancestor_of(&grandchild));
        assert!(child.is_ancestor_of(&grandchild));
        assert!(!child.is_ancestor_of(&parent));
        assert!(!grandchild.is_ancestor_of(&parent));
    }

    #[test]
    fn test_folder_parent_path() {
        let root = create_test_folder("/documents", None);
        assert_eq!(root.parent_path(), None);

        let child = create_test_folder("/documents/work", Some(Uuid::new_v4()));
        assert_eq!(child.parent_path(), Some("/documents".to_string()));

        let grandchild = create_test_folder("/documents/work/projects", Some(Uuid::new_v4()));
        assert_eq!(grandchild.parent_path(), Some("/documents/work".to_string()));
    }
}
