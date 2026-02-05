//! Usage tracking models.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

/// Database row for usage records.
#[derive(Debug, sqlx::FromRow)]
pub struct UsageRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub period_start: NaiveDate,
    pub period_type: String,
    pub downloads_count: i32,
    pub bytes_downloaded: i64,
    pub api_requests: i32,
    pub storage_bytes_used: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UsageRow> for Usage {
    fn from(row: UsageRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            period_start: row.period_start,
            period_type: row.period_type.into(),
            downloads_count: row.downloads_count,
            bytes_downloaded: row.bytes_downloaded,
            api_requests: row.api_requests,
            storage_bytes_used: row.storage_bytes_used,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Usage record for a period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub period_start: NaiveDate,
    pub period_type: PeriodType,
    pub downloads_count: i32,
    pub bytes_downloaded: i64,
    pub api_requests: i32,
    pub storage_bytes_used: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Type of tracking period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeriodType {
    Daily,
    Monthly,
}

impl PeriodType {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Monthly => "monthly",
        }
    }
}

impl std::fmt::Display for PeriodType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for PeriodType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "monthly" => Self::Monthly,
            // "daily" and any unrecognized period type default to Daily
            _ => Self::Daily,
        }
    }
}

/// Usage summary for API response.
#[derive(Debug, Serialize)]
pub struct UsageSummary {
    /// Current daily usage.
    pub daily: UsagePeriodSummary,
    /// Current monthly usage.
    pub monthly: UsagePeriodSummary,
    /// Storage usage.
    pub storage: StorageUsageSummary,
}

/// Usage summary for a specific period.
#[derive(Debug, Serialize)]
pub struct UsagePeriodSummary {
    pub period_start: NaiveDate,
    pub period_type: PeriodType,
    pub downloads_count: i32,
    pub bytes_downloaded: i64,
    pub api_requests: i32,
}

/// Storage usage summary.
#[derive(Debug, Serialize)]
pub struct StorageUsageSummary {
    /// Total bytes used.
    pub bytes_used: i64,
    /// Storage quota in bytes.
    pub quota_bytes: i64,
    /// Usage percentage.
    pub usage_percentage: f64,
    /// Total file count.
    pub file_count: i64,
}

/// Usage limits configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageLimits {
    /// Maximum downloads per day (0 = unlimited).
    #[serde(default)]
    pub max_daily_downloads: i32,
    /// Maximum downloads per month (0 = unlimited).
    #[serde(default)]
    pub max_monthly_downloads: i32,
    /// Maximum bytes per day (0 = unlimited).
    #[serde(default)]
    pub max_daily_bytes: i64,
    /// Maximum concurrent downloads.
    #[serde(default = "default_concurrent")]
    pub max_concurrent_downloads: i32,
}

fn default_concurrent() -> i32 {
    5
}

impl Default for UsageLimits {
    fn default() -> Self {
        Self {
            max_daily_downloads: 0,
            max_monthly_downloads: 0,
            max_daily_bytes: 0,
            max_concurrent_downloads: default_concurrent(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_period_type_as_str() {
        assert_eq!(PeriodType::Daily.as_str(), "daily");
        assert_eq!(PeriodType::Monthly.as_str(), "monthly");
    }

    #[test]
    fn test_period_type_from_string() {
        assert_eq!(PeriodType::from("daily".to_string()), PeriodType::Daily);
        assert_eq!(PeriodType::from("monthly".to_string()), PeriodType::Monthly);
        assert_eq!(PeriodType::from("unknown".to_string()), PeriodType::Daily);
    }

    #[test]
    fn test_usage_limits_default() {
        let limits = UsageLimits::default();
        assert_eq!(limits.max_daily_downloads, 0);
        assert_eq!(limits.max_monthly_downloads, 0);
        assert_eq!(limits.max_daily_bytes, 0);
        assert_eq!(limits.max_concurrent_downloads, 5);
    }
}
