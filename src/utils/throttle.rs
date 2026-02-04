//! Download bandwidth throttling using token bucket algorithm.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Configuration for bandwidth throttling.
#[derive(Debug, Clone, Deserialize)]
pub struct ThrottleConfig {
    /// Per-download speed limit in bytes per second (0 = unlimited).
    #[serde(default)]
    pub per_download_bytes_per_sec: u64,
    /// Per-user bandwidth limit in bytes per second (0 = unlimited).
    #[serde(default)]
    pub per_user_bytes_per_sec: u64,
    /// Global bandwidth limit in bytes per second (0 = unlimited).
    #[serde(default)]
    pub global_bytes_per_sec: u64,
}

impl Default for ThrottleConfig {
    fn default() -> Self {
        Self {
            per_download_bytes_per_sec: 0,        // Unlimited
            per_user_bytes_per_sec: 0,             // Unlimited
            global_bytes_per_sec: 0,               // Unlimited
        }
    }
}

/// Token bucket rate limiter for bandwidth throttling.
#[derive(Debug)]
pub struct TokenBucket {
    /// Maximum tokens (burst capacity).
    capacity: u64,
    /// Tokens added per second.
    rate: u64,
    /// Current available tokens.
    tokens: f64,
    /// Last time tokens were refilled.
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a new token bucket.
    ///
    /// - `rate`: tokens per second (bytes per second)
    /// - `burst`: maximum burst size in tokens (bytes)
    #[must_use]
    pub fn new(rate: u64, burst: u64) -> Self {
        Self {
            capacity: burst,
            rate,
            tokens: burst as f64,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate as f64).min(self.capacity as f64);
        self.last_refill = now;
    }

    /// Try to consume tokens. Returns how many tokens were consumed.
    pub fn try_consume(&mut self, requested: u64) -> u64 {
        self.refill();

        let available = self.tokens as u64;
        let consumed = requested.min(available);
        self.tokens -= consumed as f64;
        consumed
    }

    /// Get the time to wait until `requested` tokens are available.
    #[must_use]
    pub fn time_to_available(&mut self, requested: u64) -> Duration {
        self.refill();

        if self.tokens >= requested as f64 {
            return Duration::ZERO;
        }

        let deficit = requested as f64 - self.tokens;
        let seconds = deficit / self.rate as f64;
        Duration::from_secs_f64(seconds)
    }

    /// Get remaining tokens.
    #[must_use]
    pub fn available(&mut self) -> u64 {
        self.refill();
        self.tokens as u64
    }
}

/// Shared bandwidth throttler that can be used across async tasks.
#[derive(Clone)]
pub struct BandwidthThrottle {
    bucket: Arc<Mutex<TokenBucket>>,
    rate: u64,
}

impl BandwidthThrottle {
    /// Create a new bandwidth throttle.
    ///
    /// - `bytes_per_sec`: maximum bytes per second (0 = unlimited)
    #[must_use]
    pub fn new(bytes_per_sec: u64) -> Self {
        // Burst size = 1 second worth of data, minimum 64KB
        let burst = bytes_per_sec.max(65536);
        Self {
            bucket: Arc::new(Mutex::new(TokenBucket::new(bytes_per_sec, burst))),
            rate: bytes_per_sec,
        }
    }

    /// Check if throttling is enabled (rate > 0).
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.rate > 0
    }

    /// Consume bytes, waiting if necessary.
    pub async fn consume(&self, bytes: u64) {
        if !self.is_enabled() {
            return;
        }

        let mut remaining = bytes;
        while remaining > 0 {
            let consumed = {
                let mut bucket = self.bucket.lock().await;
                bucket.try_consume(remaining)
            };

            remaining -= consumed;

            if remaining > 0 {
                // Wait for tokens to become available
                let wait_time = {
                    let mut bucket = self.bucket.lock().await;
                    bucket.time_to_available(remaining.min(self.rate))
                };

                if wait_time > Duration::ZERO {
                    tokio::time::sleep(wait_time).await;
                }
            }
        }
    }

    /// Get current available bandwidth.
    pub async fn available_bytes(&self) -> u64 {
        let mut bucket = self.bucket.lock().await;
        bucket.available()
    }
}

/// Bandwidth statistics for monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthStats {
    /// Configured rate limit in bytes per second.
    pub rate_limit_bytes_per_sec: u64,
    /// Currently available burst capacity.
    pub available_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new(1000, 5000);
        assert_eq!(bucket.capacity, 5000);
        assert_eq!(bucket.rate, 1000);
    }

    #[test]
    fn test_token_bucket_consume() {
        let mut bucket = TokenBucket::new(1000, 5000);
        // Initial tokens = capacity = 5000
        let consumed = bucket.try_consume(3000);
        assert_eq!(consumed, 3000);

        let consumed = bucket.try_consume(3000);
        assert_eq!(consumed, 2000); // Only 2000 left
    }

    #[test]
    fn test_token_bucket_time_to_available() {
        let mut bucket = TokenBucket::new(1000, 1000);
        bucket.try_consume(1000); // Empty the bucket

        let wait = bucket.time_to_available(500);
        assert!(wait.as_secs_f64() > 0.0);
        assert!(wait.as_secs_f64() <= 0.6); // ~0.5 seconds for 500 tokens at 1000/sec
    }

    #[test]
    fn test_token_bucket_zero_available() {
        let mut bucket = TokenBucket::new(100, 100);
        let consumed = bucket.try_consume(100);
        assert_eq!(consumed, 100);

        let consumed = bucket.try_consume(50);
        assert_eq!(consumed, 0);
    }

    #[test]
    fn test_bandwidth_throttle_disabled() {
        let throttle = BandwidthThrottle::new(0);
        assert!(!throttle.is_enabled());
    }

    #[test]
    fn test_bandwidth_throttle_enabled() {
        let throttle = BandwidthThrottle::new(1_000_000);
        assert!(throttle.is_enabled());
    }

    #[tokio::test]
    async fn test_bandwidth_throttle_consume_unlimited() {
        let throttle = BandwidthThrottle::new(0);
        // Should return immediately for unlimited
        throttle.consume(1_000_000).await;
    }

    #[test]
    fn test_throttle_config_default() {
        let config = ThrottleConfig::default();
        assert_eq!(config.per_download_bytes_per_sec, 0);
        assert_eq!(config.per_user_bytes_per_sec, 0);
        assert_eq!(config.global_bytes_per_sec, 0);
    }
}
