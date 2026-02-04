//! Admin-related models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// System-wide admin statistics.
#[derive(Debug, Serialize)]
pub struct AdminStats {
    pub users: UserStats,
    pub downloads: DownloadStats,
    pub storage: StorageStats,
    pub jobs: JobStats,
}

#[derive(Debug, Serialize)]
pub struct UserStats {
    pub total: i64,
    pub active_today: i64,
    pub active_this_week: i64,
    pub new_this_month: i64,
}

#[derive(Debug, Serialize)]
pub struct DownloadStats {
    pub total: i64,
    pub completed: i64,
    pub failed: i64,
    pub in_progress: i64,
    pub total_bytes: i64,
}

#[derive(Debug, Serialize)]
pub struct StorageStats {
    pub total_bytes_used: i64,
    pub total_files: i64,
}

#[derive(Debug, Serialize)]
pub struct JobStats {
    pub pending: i64,
    pub completed: i64,
    pub failed: i64,
}

/// Admin user listing entry.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminUserRow {
    pub id: uuid::Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub is_admin: bool,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to update a user from admin panel.
#[derive(Debug, Deserialize, Serialize)]
pub struct AdminUpdateUserRequest {
    pub is_admin: Option<bool>,
    pub email_verified: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_stats_serialization() {
        let stats = AdminStats {
            users: UserStats {
                total: 100,
                active_today: 50,
                active_this_week: 80,
                new_this_month: 10,
            },
            downloads: DownloadStats {
                total: 5000,
                completed: 4500,
                failed: 100,
                in_progress: 50,
                total_bytes: 1_000_000_000,
            },
            storage: StorageStats {
                total_bytes_used: 500_000_000,
                total_files: 4000,
            },
            jobs: JobStats {
                pending: 10,
                completed: 4900,
                failed: 50,
            },
        };

        let json = serde_json::to_string(&stats);
        assert!(json.is_ok());
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("\"total\":100"));
        assert!(json_str.contains("\"completed\":4500"));
    }

    #[test]
    fn test_admin_update_user_request_deserialization() {
        let json = r#"{"is_admin": true}"#;
        let req: Result<AdminUpdateUserRequest, _> = serde_json::from_str(json);
        assert!(req.is_ok());
        let req = req.unwrap_or_else(|_| AdminUpdateUserRequest {
            is_admin: None,
            email_verified: None,
        });
        assert_eq!(req.is_admin, Some(true));
        assert!(req.email_verified.is_none());
    }
}
