//! Audit log repository functions.

use crate::models::{AuditLog, AuditLogQuery, CreateAuditLog};
use anyhow::{Context, Result};
use sqlx::PgPool;
use std::fmt::Write;

/// Insert an audit log entry.
#[tracing::instrument(name = "Create audit log", skip(pool))]
pub async fn create_audit_log(log: &CreateAuditLog, pool: &PgPool) -> Result<AuditLog> {
    let row = sqlx::query_as::<_, AuditLog>(
        r"
        INSERT INTO audit_logs (user_id, action, resource_type, resource_id, old_values, new_values, ip_address, user_agent)
        VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8)
        RETURNING id, user_id, action, resource_type, resource_id, old_values, new_values,
                  host(ip_address)::text as ip_address, user_agent, created_at
        ",
    )
    .bind(log.user_id)
    .bind(&log.action)
    .bind(&log.resource_type)
    .bind(log.resource_id)
    .bind(&log.old_values)
    .bind(&log.new_values)
    .bind(&log.ip_address)
    .bind(&log.user_agent)
    .fetch_one(pool)
    .await
    .context("Failed to create audit log")?;

    Ok(row)
}

/// Query audit logs with optional filters.
#[tracing::instrument(name = "List audit logs", skip(pool))]
pub async fn list_audit_logs(query: &AuditLogQuery, pool: &PgPool) -> Result<Vec<AuditLog>> {
    // Build dynamic query
    let mut sql = String::from(
        r"
        SELECT id, user_id, action, resource_type, resource_id, old_values, new_values,
               host(ip_address)::text as ip_address, user_agent, created_at
        FROM audit_logs
        WHERE 1=1
        ",
    );

    let mut param_idx = 0;

    if query.user_id.is_some() {
        param_idx += 1;
        let _ = write!(sql, " AND user_id = ${param_idx}");
    }
    if query.action.is_some() {
        param_idx += 1;
        let _ = write!(sql, " AND action = ${param_idx}");
    }
    if query.resource_type.is_some() {
        param_idx += 1;
        let _ = write!(sql, " AND resource_type = ${param_idx}");
    }

    param_idx += 1;
    let limit_idx = param_idx;
    param_idx += 1;
    let offset_idx = param_idx;

    let _ = write!(
        sql,
        " ORDER BY created_at DESC LIMIT ${limit_idx} OFFSET ${offset_idx}"
    );

    let mut q = sqlx::query_as::<_, AuditLog>(&sql);

    if let Some(user_id) = query.user_id {
        q = q.bind(user_id);
    }
    if let Some(ref action) = query.action {
        q = q.bind(action);
    }
    if let Some(ref resource_type) = query.resource_type {
        q = q.bind(resource_type);
    }

    q = q.bind(query.limit());
    q = q.bind(query.offset());

    let logs = q
        .fetch_all(pool)
        .await
        .context("Failed to list audit logs")?;

    Ok(logs)
}

/// Count total audit logs matching filters.
#[tracing::instrument(name = "Count audit logs", skip(pool))]
pub async fn count_audit_logs(query: &AuditLogQuery, pool: &PgPool) -> Result<i64> {
    let mut sql = String::from("SELECT COUNT(*) FROM audit_logs WHERE 1=1");
    let mut param_idx = 0;

    if query.user_id.is_some() {
        param_idx += 1;
        let _ = write!(sql, " AND user_id = ${param_idx}");
    }
    if query.action.is_some() {
        param_idx += 1;
        let _ = write!(sql, " AND action = ${param_idx}");
    }
    if query.resource_type.is_some() {
        param_idx += 1;
        let _ = write!(sql, " AND resource_type = ${param_idx}");
    }

    let mut q = sqlx::query_scalar::<_, i64>(&sql);

    if let Some(user_id) = query.user_id {
        q = q.bind(user_id);
    }
    if let Some(ref action) = query.action {
        q = q.bind(action);
    }
    if let Some(ref resource_type) = query.resource_type {
        q = q.bind(resource_type);
    }

    let count = q
        .fetch_one(pool)
        .await
        .context("Failed to count audit logs")?;
    Ok(count)
}
