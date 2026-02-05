//! Audit log models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Audit log record from the database.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request to create an audit log entry.
#[derive(Debug, Clone)]
pub struct CreateAuditLog {
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Query params for listing audit logs.
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub user_id: Option<Uuid>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl AuditLogQuery {
    #[must_use]
    pub fn limit(&self) -> i64 {
        self.per_page.unwrap_or(50).min(100)
    }

    #[must_use]
    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(1).max(1);
        (page - 1) * self.limit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_query_defaults() {
        let query = AuditLogQuery {
            user_id: None,
            action: None,
            resource_type: None,
            page: None,
            per_page: None,
        };

        assert_eq!(query.limit(), 50);
        assert_eq!(query.offset(), 0);
    }

    #[test]
    fn test_audit_log_query_pagination() {
        let query = AuditLogQuery {
            user_id: None,
            action: None,
            resource_type: None,
            page: Some(3),
            per_page: Some(20),
        };

        assert_eq!(query.limit(), 20);
        assert_eq!(query.offset(), 40);
    }

    #[test]
    fn test_audit_log_query_max_limit() {
        let query = AuditLogQuery {
            user_id: None,
            action: None,
            resource_type: None,
            page: Some(1),
            per_page: Some(500),
        };

        assert_eq!(query.limit(), 100); // capped at 100
    }

    #[test]
    fn test_create_audit_log() {
        let log = CreateAuditLog {
            user_id: Some(Uuid::new_v4()),
            action: "update".to_string(),
            resource_type: "user".to_string(),
            resource_id: Some(Uuid::new_v4()),
            old_values: Some(serde_json::json!({"is_admin": false})),
            new_values: Some(serde_json::json!({"is_admin": true})),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("TestAgent/1.0".to_string()),
        };

        assert_eq!(log.action, "update");
        assert_eq!(log.resource_type, "user");
    }
}
