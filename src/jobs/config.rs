//! Job queue configuration.

use serde::Deserialize;

/// Main queue configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct QueueConfig {
    /// Redis Streams configuration.
    pub redis_streams: RedisStreamsConfig,
    /// Worker pool configuration.
    pub worker_pool: WorkerPoolConfig,
    /// Job history configuration.
    #[serde(default)]
    pub history: HistoryConfig,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            redis_streams: RedisStreamsConfig::default(),
            worker_pool: WorkerPoolConfig::default(),
            history: HistoryConfig::default(),
        }
    }
}

/// Redis Streams specific configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RedisStreamsConfig {
    /// Redis connection URL (defaults to redis_uri from main config).
    #[serde(default = "default_redis_url")]
    pub url: String,
    /// Prefix for stream keys (e.g., "jobs" -> "jobs:download").
    #[serde(default = "default_stream_prefix")]
    pub stream_prefix: String,
    /// Consumer group name.
    #[serde(default = "default_consumer_group")]
    pub consumer_group: String,
    /// Prefix for consumer/worker names.
    #[serde(default = "default_consumer_name_prefix")]
    pub consumer_name_prefix: String,
    /// Block timeout in milliseconds for XREADGROUP.
    #[serde(default = "default_block_ms")]
    pub block_ms: u64,
    /// Maximum retry attempts for failed jobs.
    #[serde(default = "default_max_retries")]
    pub max_retries: i32,
    /// Delay between retries in milliseconds.
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
    /// Timeout for pending messages before they can be claimed (milliseconds).
    #[serde(default = "default_pending_timeout_ms")]
    pub pending_timeout_ms: u64,
    /// Maximum stream length (oldest entries trimmed).
    #[serde(default = "default_max_stream_length")]
    pub max_stream_length: u64,
}

fn default_redis_url() -> String {
    "redis://localhost:6379".to_string()
}

fn default_stream_prefix() -> String {
    "jobs".to_string()
}

fn default_consumer_group() -> String {
    "workers".to_string()
}

fn default_consumer_name_prefix() -> String {
    "worker".to_string()
}

fn default_block_ms() -> u64 {
    5000
}

fn default_max_retries() -> i32 {
    3
}

fn default_retry_delay_ms() -> u64 {
    5000
}

fn default_pending_timeout_ms() -> u64 {
    300000 // 5 minutes
}

fn default_max_stream_length() -> u64 {
    10000
}

impl Default for RedisStreamsConfig {
    fn default() -> Self {
        Self {
            url: default_redis_url(),
            stream_prefix: default_stream_prefix(),
            consumer_group: default_consumer_group(),
            consumer_name_prefix: default_consumer_name_prefix(),
            block_ms: default_block_ms(),
            max_retries: default_max_retries(),
            retry_delay_ms: default_retry_delay_ms(),
            pending_timeout_ms: default_pending_timeout_ms(),
            max_stream_length: default_max_stream_length(),
        }
    }
}

impl RedisStreamsConfig {
    /// Get the stream key for a job type.
    pub fn stream_key(&self, job_type: &str) -> String {
        format!("{}:{}", self.stream_prefix, job_type)
    }

    /// Get the dead letter stream key.
    pub fn dead_letter_key(&self) -> String {
        format!("{}:dead_letter", self.stream_prefix)
    }
}

/// Worker pool configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkerPoolConfig {
    /// Number of workers to spawn.
    #[serde(default = "default_num_workers")]
    pub num_workers: usize,
    /// Prefix for worker names.
    #[serde(default = "default_worker_name_prefix")]
    pub worker_name_prefix: String,
    /// Shutdown timeout in seconds.
    #[serde(default = "default_shutdown_timeout_secs")]
    pub shutdown_timeout_secs: u64,
}

fn default_num_workers() -> usize {
    4
}

fn default_worker_name_prefix() -> String {
    "worker".to_string()
}

fn default_shutdown_timeout_secs() -> u64 {
    30
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            num_workers: default_num_workers(),
            worker_name_prefix: default_worker_name_prefix(),
            shutdown_timeout_secs: default_shutdown_timeout_secs(),
        }
    }
}

/// Job history configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct HistoryConfig {
    /// Whether to record job history in PostgreSQL.
    #[serde(default = "default_history_enabled")]
    pub enabled: bool,
    /// Number of days to retain job history.
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}

fn default_history_enabled() -> bool {
    true
}

fn default_retention_days() -> u32 {
    90
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            enabled: default_history_enabled(),
            retention_days: default_retention_days(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = QueueConfig::default();
        assert_eq!(config.redis_streams.stream_prefix, "jobs");
        assert_eq!(config.worker_pool.num_workers, 4);
        assert!(config.history.enabled);
    }

    #[test]
    fn test_stream_key() {
        let config = RedisStreamsConfig::default();
        assert_eq!(config.stream_key("download"), "jobs:download");
        assert_eq!(config.dead_letter_key(), "jobs:dead_letter");
    }
}
