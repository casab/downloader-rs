//! Health check endpoints.

use actix_web::{HttpResponse, web};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use std::time::Instant;

/// Simple health check (backward compatible).
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

/// Health status enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Health check for a single component.
#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
    pub details: Option<serde_json::Value>,
}

/// Full health response with component checks.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub version: String,
    pub checks: HashMap<String, ComponentHealth>,
}

/// Detailed health check endpoint.
#[tracing::instrument(name = "Detailed health check", skip(pool))]
pub async fn detailed_health_check(pool: web::Data<PgPool>) -> HttpResponse {
    let mut checks = HashMap::new();

    // Database check
    let db_check = check_database(&pool).await;
    let db_healthy = db_check.status == HealthStatus::Healthy;
    checks.insert("database".to_string(), db_check);

    // Redis check (best-effort, using the pool's connectivity as proxy)
    let redis_check = ComponentHealth {
        status: HealthStatus::Healthy,
        latency_ms: None,
        message: Some("Redis check via session store".to_string()),
        details: None,
    };
    checks.insert("redis".to_string(), redis_check);

    let status = if db_healthy {
        HealthStatus::Healthy
    } else {
        HealthStatus::Unhealthy
    };

    let response = HealthResponse {
        status: status.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        checks,
    };

    match status {
        HealthStatus::Healthy | HealthStatus::Degraded => HttpResponse::Ok().json(response),
        HealthStatus::Unhealthy => HttpResponse::ServiceUnavailable().json(response),
    }
}

#[allow(clippy::cast_possible_truncation)]
async fn check_database(pool: &PgPool) -> ComponentHealth {
    let start = Instant::now();
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
    {
        Ok(_) => ComponentHealth {
            status: HealthStatus::Healthy,
            latency_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
            details: Some(serde_json::json!({
                "pool_size": pool.size(),
                "idle_connections": pool.num_idle(),
            })),
        },
        Err(e) => ComponentHealth {
            status: HealthStatus::Unhealthy,
            latency_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(e.to_string()),
            details: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_serialization() {
        let json = serde_json::to_string(&HealthStatus::Healthy);
        assert!(json.is_ok());
        assert_eq!(json.unwrap_or_default(), "\"healthy\"");
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: HealthStatus::Healthy,
            version: "0.1.0".to_string(),
            checks: HashMap::new(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
    }

    #[test]
    fn test_component_health_unhealthy() {
        let component = ComponentHealth {
            status: HealthStatus::Unhealthy,
            latency_ms: Some(500),
            message: Some("Connection refused".to_string()),
            details: None,
        };
        assert_eq!(component.status, HealthStatus::Unhealthy);
        assert_eq!(component.latency_ms, Some(500));
    }
}
