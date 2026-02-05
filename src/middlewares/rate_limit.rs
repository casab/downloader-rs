//! Rate limiting middleware using Redis-backed token bucket algorithm.

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::HttpMessage;
use actix_web::HttpResponse;
use anyhow::Result;
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Rate limiter using Redis for distributed rate limiting.
pub struct RateLimiter {
    client: Arc<redis::Client>,
    config: RateLimitConfig,
}

/// Rate limit configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per minute.
    #[serde(default = "default_rpm")]
    pub requests_per_minute: u32,
    /// Maximum requests per hour.
    #[serde(default = "default_rph")]
    pub requests_per_hour: u32,
    /// Burst size (max tokens in bucket).
    #[serde(default = "default_burst")]
    pub burst_size: u32,
    /// Whether rate limiting is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_rpm() -> u32 { 60 }
fn default_rph() -> u32 { 1000 }
fn default_burst() -> u32 { 10 }
fn default_enabled() -> bool { true }

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: default_rpm(),
            requests_per_hour: default_rph(),
            burst_size: default_burst(),
            enabled: default_enabled(),
        }
    }
}

/// Result of a rate limit check.
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitResult {
    /// Whether the request is allowed.
    pub allowed: bool,
    /// Remaining requests in the current window.
    pub remaining: u32,
    /// When the rate limit resets.
    pub reset_at: DateTime<Utc>,
    /// How long to wait before retrying (if rate limited).
    pub retry_after: Option<Duration>,
    /// Total limit.
    pub limit: u32,
}

/// Rate limit error.
#[derive(Debug, Clone)]
pub struct RateLimitError {
    pub result: RateLimitResult,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rate limit exceeded. Retry after {:?}", self.result.retry_after)
    }
}

impl std::error::Error for RateLimitError {}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new(client: Arc<redis::Client>, config: RateLimitConfig) -> Self {
        Self { client, config }
    }

    /// Check rate limit for a key using sliding window counter.
    pub async fn check(&self, key: &str) -> Result<RateLimitResult> {
        if !self.config.enabled {
            return Ok(RateLimitResult {
                allowed: true,
                remaining: self.config.requests_per_minute,
                reset_at: Utc::now() + chrono::Duration::minutes(1),
                retry_after: None,
                limit: self.config.requests_per_minute,
            });
        }

        let mut conn = self.client
            .get_multiplexed_async_connection()
            .await?;

        let now = Utc::now();
        let window_key = format!("rate_limit:{}:{}", key, now.timestamp() / 60);
        let window_start = (now.timestamp() / 60) * 60;
        let reset_at = DateTime::from_timestamp(window_start + 60, 0)
            .unwrap_or(now);

        // Increment counter with TTL
        let count: u32 = redis::cmd("INCR")
            .arg(&window_key)
            .query_async(&mut conn)
            .await?;

        // Set expiry on first request in window
        if count == 1 {
            let _: () = conn.expire(&window_key, 120).await?; // 2 minute TTL for safety
        }

        let allowed = count <= self.config.requests_per_minute;
        let remaining = if allowed {
            self.config.requests_per_minute - count
        } else {
            0
        };

        let retry_after = if allowed {
            None
        } else {
            let seconds_until_reset = (reset_at - now).num_seconds().max(1) as u64;
            Some(Duration::from_secs(seconds_until_reset))
        };

        Ok(RateLimitResult {
            allowed,
            remaining,
            reset_at,
            retry_after,
            limit: self.config.requests_per_minute,
        })
    }

    /// Build HTTP response headers for rate limiting.
    pub fn build_headers(result: &RateLimitResult) -> Vec<(&'static str, String)> {
        let mut headers = vec![
            ("X-RateLimit-Limit", result.limit.to_string()),
            ("X-RateLimit-Remaining", result.remaining.to_string()),
            ("X-RateLimit-Reset", result.reset_at.timestamp().to_string()),
        ];

        if let Some(retry_after) = result.retry_after {
            headers.push(("Retry-After", retry_after.as_secs().to_string()));
        }

        headers
    }

    /// Build a 429 Too Many Requests response.
    pub fn too_many_requests(result: &RateLimitResult) -> HttpResponse {
        let mut response = HttpResponse::TooManyRequests();

        for (name, value) in Self::build_headers(result) {
            response.insert_header((name, value));
        }

        response.json(serde_json::json!({
            "error": "Too Many Requests",
            "message": "Rate limit exceeded",
            "retry_after": result.retry_after.map(|d| d.as_secs()),
        }))
    }
}

/// Actix-web middleware function for rate limiting.
///
/// Uses user ID (if available from auth middleware) or client IP as the rate limit key.
/// If the rate limiter is not configured or Redis is unavailable, requests pass through.
pub async fn rate_limit_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> std::result::Result<ServiceResponse<impl MessageBody>, actix_web::Error> {
    // Get rate limiter from app data
    let Some(limiter) = req.app_data::<actix_web::web::Data<RateLimiter>>() else {
        return next.call(req).await.map(ServiceResponse::map_into_left_body);
    };
    let limiter = limiter.clone();

    // Build rate limit key: prefer user ID (set by auth middleware), fall back to IP
    let key = req
        .extensions()
        .get::<super::UserId>()
        .map(|uid| format!("user:{}", uid.0))
        .unwrap_or_else(|| {
            req.peer_addr()
                .map(|addr| format!("ip:{}", addr.ip()))
                .unwrap_or_else(|| "ip:unknown".to_string())
        });

    match limiter.check(&key).await {
        Ok(result) if !result.allowed => {
            Ok(req.into_response(RateLimiter::too_many_requests(&result)).map_into_right_body())
        }
        Ok(result) => {
            let mut response = next.call(req).await?;
            for (name, value) in RateLimiter::build_headers(&result) {
                if let Ok(hv) = actix_web::http::header::HeaderValue::from_str(&value) {
                    response.headers_mut().insert(
                        actix_web::http::header::HeaderName::from_static(name),
                        hv,
                    );
                }
            }
            Ok(response.map_into_left_body())
        }
        Err(e) => {
            // If rate limiter fails (e.g. Redis down), allow the request through
            tracing::warn!("Rate limiter error: {}, allowing request", e);
            next.call(req).await.map(ServiceResponse::map_into_left_body)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_minute, 60);
        assert_eq!(config.requests_per_hour, 1000);
        assert_eq!(config.burst_size, 10);
        assert!(config.enabled);
    }

    #[test]
    fn test_rate_limit_result_allowed() {
        let result = RateLimitResult {
            allowed: true,
            remaining: 59,
            reset_at: Utc::now(),
            retry_after: None,
            limit: 60,
        };

        assert!(result.allowed);
        assert_eq!(result.remaining, 59);
        assert!(result.retry_after.is_none());
    }

    #[test]
    fn test_rate_limit_result_denied() {
        let result = RateLimitResult {
            allowed: false,
            remaining: 0,
            reset_at: Utc::now() + chrono::Duration::seconds(30),
            retry_after: Some(Duration::from_secs(30)),
            limit: 60,
        };

        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
        assert!(result.retry_after.is_some());
    }

    #[test]
    fn test_build_headers() {
        let result = RateLimitResult {
            allowed: true,
            remaining: 50,
            reset_at: Utc::now(),
            retry_after: None,
            limit: 60,
        };

        let headers = RateLimiter::build_headers(&result);
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0].0, "X-RateLimit-Limit");
        assert_eq!(headers[0].1, "60");
    }

    #[test]
    fn test_build_headers_with_retry_after() {
        let result = RateLimitResult {
            allowed: false,
            remaining: 0,
            reset_at: Utc::now(),
            retry_after: Some(Duration::from_secs(45)),
            limit: 60,
        };

        let headers = RateLimiter::build_headers(&result);
        assert_eq!(headers.len(), 4);
        assert_eq!(headers[3].0, "Retry-After");
        assert_eq!(headers[3].1, "45");
    }
}
