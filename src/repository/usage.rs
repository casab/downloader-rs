//! Usage tracking repository.

use crate::{
    middlewares::UserId,
    models::{PeriodType, StorageUsageSummary, Usage, UsagePeriodSummary, UsageRow, UsageSummary},
};
use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, Utc};
use sqlx::PgPool;

/// Get or create the current daily usage record for a user.
#[tracing::instrument(name = "Get or create daily usage", skip(pool))]
pub async fn get_or_create_daily_usage(user_id: &UserId, pool: &PgPool) -> Result<Usage> {
    let today = Utc::now().date_naive();
    get_or_create_usage(user_id, today, &PeriodType::Daily, pool).await
}

/// Get or create the current monthly usage record for a user.
#[tracing::instrument(name = "Get or create monthly usage", skip(pool))]
pub async fn get_or_create_monthly_usage(user_id: &UserId, pool: &PgPool) -> Result<Usage> {
    let today = Utc::now().date_naive();
    let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    get_or_create_usage(user_id, month_start, &PeriodType::Monthly, pool).await
}

/// Get or create a usage record.
async fn get_or_create_usage(
    user_id: &UserId,
    period_start: NaiveDate,
    period_type: &PeriodType,
    pool: &PgPool,
) -> Result<Usage> {
    let row = sqlx::query_as::<_, UsageRow>(
        r"
        INSERT INTO usage (id, user_id, period_start, period_type)
        VALUES (gen_random_uuid(), $1, $2, $3)
        ON CONFLICT (user_id, period_start, period_type) DO UPDATE
        SET updated_at = NOW()
        RETURNING id, user_id, period_start, period_type, downloads_count,
                  bytes_downloaded, api_requests, storage_bytes_used, created_at, updated_at
        ",
    )
    .bind(user_id.0)
    .bind(period_start)
    .bind(period_type.as_str())
    .fetch_one(pool)
    .await
    .context("Failed to get or create usage record")?;

    Ok(Usage::from(row))
}

/// Increment download count for current day and month.
#[tracing::instrument(name = "Increment download count", skip(pool))]
pub async fn increment_download_count(user_id: &UserId, bytes: i64, pool: &PgPool) -> Result<()> {
    let today = Utc::now().date_naive();
    let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);

    // Update daily
    sqlx::query(
        r"
        INSERT INTO usage (id, user_id, period_start, period_type, downloads_count, bytes_downloaded)
        VALUES (gen_random_uuid(), $1, $2, 'daily', 1, $3)
        ON CONFLICT (user_id, period_start, period_type) DO UPDATE
        SET downloads_count = usage.downloads_count + 1,
            bytes_downloaded = usage.bytes_downloaded + $3,
            updated_at = NOW()
        ",
    )
    .bind(user_id.0)
    .bind(today)
    .bind(bytes)
    .execute(pool)
    .await
    .context("Failed to increment daily download count")?;

    // Update monthly
    sqlx::query(
        r"
        INSERT INTO usage (id, user_id, period_start, period_type, downloads_count, bytes_downloaded)
        VALUES (gen_random_uuid(), $1, $2, 'monthly', 1, $3)
        ON CONFLICT (user_id, period_start, period_type) DO UPDATE
        SET downloads_count = usage.downloads_count + 1,
            bytes_downloaded = usage.bytes_downloaded + $3,
            updated_at = NOW()
        ",
    )
    .bind(user_id.0)
    .bind(month_start)
    .bind(bytes)
    .execute(pool)
    .await
    .context("Failed to increment monthly download count")?;

    Ok(())
}

/// Increment API request count.
#[tracing::instrument(name = "Increment API request count", skip(pool))]
pub async fn increment_api_requests(user_id: &UserId, pool: &PgPool) -> Result<()> {
    let today = Utc::now().date_naive();

    sqlx::query(
        r"
        INSERT INTO usage (id, user_id, period_start, period_type, api_requests)
        VALUES (gen_random_uuid(), $1, $2, 'daily', 1)
        ON CONFLICT (user_id, period_start, period_type) DO UPDATE
        SET api_requests = usage.api_requests + 1,
            updated_at = NOW()
        ",
    )
    .bind(user_id.0)
    .bind(today)
    .execute(pool)
    .await
    .context("Failed to increment API request count")?;

    Ok(())
}

/// Update storage bytes used.
#[tracing::instrument(name = "Update storage usage", skip(pool))]
pub async fn update_storage_usage(
    user_id: &UserId,
    storage_bytes: i64,
    pool: &PgPool,
) -> Result<()> {
    let today = Utc::now().date_naive();

    sqlx::query(
        r"
        INSERT INTO usage (id, user_id, period_start, period_type, storage_bytes_used)
        VALUES (gen_random_uuid(), $1, $2, 'daily', $3)
        ON CONFLICT (user_id, period_start, period_type) DO UPDATE
        SET storage_bytes_used = $3,
            updated_at = NOW()
        ",
    )
    .bind(user_id.0)
    .bind(today)
    .bind(storage_bytes)
    .execute(pool)
    .await
    .context("Failed to update storage usage")?;

    Ok(())
}

/// Get the usage summary for a user.
#[tracing::instrument(name = "Get usage summary", skip(pool))]
pub async fn get_usage_summary(
    user_id: &UserId,
    quota_bytes: i64,
    pool: &PgPool,
) -> Result<UsageSummary> {
    let daily = get_or_create_daily_usage(user_id, pool).await?;
    let monthly = get_or_create_monthly_usage(user_id, pool).await?;

    // Get total storage used (sum of all file sizes for the user)
    let storage_bytes: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_bytes), 0) FROM downloads WHERE user_id = $1 AND status = 'COMPLETED'",
    )
    .bind(user_id.0)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let file_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM downloads WHERE user_id = $1 AND status = 'COMPLETED'",
    )
    .bind(user_id.0)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    #[allow(clippy::cast_precision_loss)]
    let usage_percentage = if quota_bytes > 0 {
        (storage_bytes as f64 / quota_bytes as f64) * 100.0
    } else {
        0.0
    };

    Ok(UsageSummary {
        daily: UsagePeriodSummary {
            period_start: daily.period_start,
            period_type: PeriodType::Daily,
            downloads_count: daily.downloads_count,
            bytes_downloaded: daily.bytes_downloaded,
            api_requests: daily.api_requests,
        },
        monthly: UsagePeriodSummary {
            period_start: monthly.period_start,
            period_type: PeriodType::Monthly,
            downloads_count: monthly.downloads_count,
            bytes_downloaded: monthly.bytes_downloaded,
            api_requests: monthly.api_requests,
        },
        storage: StorageUsageSummary {
            bytes_used: storage_bytes,
            quota_bytes,
            usage_percentage,
            file_count,
        },
    })
}
