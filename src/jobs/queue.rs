//! Redis Streams job queue implementation.

use crate::jobs::{Job, JobResult, JobStatus, JobType, RedisStreamsConfig};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use redis::aio::MultiplexedConnection;
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, Client, RedisResult};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

/// Trait for job queue implementations.
#[async_trait]
pub trait JobQueue: Send + Sync {
    /// Add a job to the queue.
    async fn enqueue(&self, job: Job) -> Result<String>;

    /// Get the next job from the queue.
    async fn dequeue(&self, worker_id: &str) -> Result<Option<Job>>;

    /// Acknowledge successful job completion.
    async fn acknowledge(&self, stream_key: &str, message_id: &str) -> Result<()>;

    /// Mark a job as failed (moves to dead letter queue after max retries).
    async fn fail(&self, job: &Job, error: &str) -> Result<()>;
}

/// Redis Streams-based job queue.
pub struct RedisStreamsQueue {
    client: Client,
    config: RedisStreamsConfig,
    pool: Option<PgPool>,
}

impl RedisStreamsQueue {
    /// Create a new Redis Streams queue.
    pub fn new(config: RedisStreamsConfig) -> Result<Self> {
        let client = Client::open(config.url.as_str())
            .context("Failed to create Redis client")?;

        Ok(Self {
            client,
            config,
            pool: None,
        })
    }

    /// Create a new Redis Streams queue with PostgreSQL for job history.
    pub fn with_pool(config: RedisStreamsConfig, pool: PgPool) -> Result<Self> {
        let client = Client::open(config.url.as_str())
            .context("Failed to create Redis client")?;

        Ok(Self {
            client,
            config,
            pool: Some(pool),
        })
    }

    /// Initialize streams and consumer groups.
    pub async fn initialize(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;

        // Create streams and consumer groups for each job type
        for job_type in JobType::all() {
            let stream_key = self.config.stream_key(job_type.as_str());

            // Create consumer group (MKSTREAM creates stream if not exists)
            let result: RedisResult<()> = redis::cmd("XGROUP")
                .arg("CREATE")
                .arg(&stream_key)
                .arg(&self.config.consumer_group)
                .arg("0")
                .arg("MKSTREAM")
                .query_async(&mut conn)
                .await;

            match result {
                Ok(()) => tracing::info!("Created consumer group for stream: {}", stream_key),
                Err(e) if e.to_string().contains("BUSYGROUP") => {
                    tracing::debug!("Consumer group already exists for stream: {}", stream_key);
                }
                Err(e) => return Err(e.into()),
            }
        }

        // Create dead letter stream
        let dead_letter_key = self.config.dead_letter_key();
        let result: RedisResult<()> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(&dead_letter_key)
            .arg(&self.config.consumer_group)
            .arg("0")
            .arg("MKSTREAM")
            .query_async(&mut conn)
            .await;

        match result {
            Ok(()) => tracing::info!("Created dead letter stream: {}", dead_letter_key),
            Err(e) if e.to_string().contains("BUSYGROUP") => {
                tracing::debug!("Dead letter stream already exists");
            }
            Err(e) => return Err(e.into()),
        }

        Ok(())
    }

    /// Get a multiplexed connection.
    async fn get_connection(&self) -> Result<MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .context("Failed to get Redis connection")
    }

    /// Record job in PostgreSQL history.
    async fn record_job_created(&self, job: &Job, message_id: &str) -> Result<()> {
        let Some(pool) = &self.pool else {
            return Ok(());
        };

        sqlx::query(
            r#"
            INSERT INTO job_history (id, stream_message_id, job_type, status, payload, priority, user_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(job.id)
        .bind(message_id)
        .bind(job.job_type.as_str())
        .bind(JobStatus::Pending.to_string())
        .bind(&job.payload)
        .bind(job.priority)
        .bind(job.user_id)
        .bind(job.created_at)
        .execute(pool)
        .await
        .context("Failed to record job history")?;

        Ok(())
    }

    /// Update job status in PostgreSQL history.
    pub async fn update_job_status(
        &self,
        job_id: Uuid,
        status: JobStatus,
        worker_id: Option<&str>,
        result: Option<&JobResult>,
        error: Option<&str>,
    ) -> Result<()> {
        let Some(pool) = &self.pool else {
            return Ok(());
        };

        let now = Utc::now();
        let result_json = result.map(|r| &r.data);

        match status {
            JobStatus::Running => {
                sqlx::query(
                    r#"
                    UPDATE job_history
                    SET status = $2, worker_id = $3, started_at = $4, attempts = attempts + 1
                    WHERE id = $1
                    "#,
                )
                .bind(job_id)
                .bind(status.to_string())
                .bind(worker_id)
                .bind(now)
                .execute(pool)
                .await?;
            }
            JobStatus::Completed => {
                sqlx::query(
                    r#"
                    UPDATE job_history
                    SET status = $2, result = $3, completed_at = $4
                    WHERE id = $1
                    "#,
                )
                .bind(job_id)
                .bind(status.to_string())
                .bind(result_json)
                .bind(now)
                .execute(pool)
                .await?;
            }
            JobStatus::Failed => {
                sqlx::query(
                    r#"
                    UPDATE job_history
                    SET status = $2, error_message = $3, completed_at = $4
                    WHERE id = $1
                    "#,
                )
                .bind(job_id)
                .bind(status.to_string())
                .bind(error)
                .bind(now)
                .execute(pool)
                .await?;
            }
            _ => {
                sqlx::query(
                    r#"
                    UPDATE job_history
                    SET status = $2
                    WHERE id = $1
                    "#,
                )
                .bind(job_id)
                .bind(status.to_string())
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Parse a job from Redis stream message fields.
    fn parse_job(&self, stream_key: &str, message_id: &str, fields: &HashMap<String, redis::Value>) -> Result<Job> {
        let get_string = |key: &str| -> Result<String> {
            fields
                .get(key)
                .and_then(|v| match v {
                    redis::Value::BulkString(bytes) => String::from_utf8(bytes.clone()).ok(),
                    redis::Value::SimpleString(s) => Some(s.clone()),
                    _ => None,
                })
                .ok_or_else(|| anyhow::anyhow!("Missing or invalid field: {}", key))
        };

        let id: Uuid = get_string("id")?.parse()?;
        let job_type_str = get_string("type")?;
        let payload_str = get_string("payload")?;
        let priority: i32 = get_string("priority")?.parse().unwrap_or(0);
        let attempts: i32 = get_string("attempts").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
        let max_retries: i32 = get_string("max_retries").ok().and_then(|s| s.parse().ok()).unwrap_or(self.config.max_retries);
        let created_at_ms: i64 = get_string("created_at")?.parse()?;

        let job_type = match job_type_str.as_str() {
            "download" => JobType::Download,
            "email" => JobType::Email,
            "cleanup" => JobType::Cleanup,
            "notification" => JobType::Notification,
            "webhook" => JobType::Webhook,
            _ => return Err(anyhow::anyhow!("Unknown job type: {}", job_type_str)),
        };

        let payload: serde_json::Value = serde_json::from_str(&payload_str)?;
        let created_at = chrono::DateTime::from_timestamp_millis(created_at_ms)
            .unwrap_or_else(Utc::now);

        let user_id = get_string("user_id")
            .ok()
            .and_then(|s| s.parse().ok());

        Ok(Job {
            id,
            job_type,
            payload,
            priority,
            attempts,
            max_retries,
            stream_key: Some(stream_key.to_string()),
            message_id: Some(message_id.to_string()),
            created_at,
            user_id,
        })
    }

    /// Claim pending messages from dead/slow consumers.
    async fn claim_pending_job(
        &self,
        conn: &mut MultiplexedConnection,
        worker_id: &str,
    ) -> Result<Option<Job>> {
        for job_type in JobType::all() {
            let stream_key = self.config.stream_key(job_type.as_str());

            // XPENDING to find stuck messages
            let pending: redis::Value = redis::cmd("XPENDING")
                .arg(&stream_key)
                .arg(&self.config.consumer_group)
                .arg("-")
                .arg("+")
                .arg(1)
                .query_async(conn)
                .await?;

            // Parse pending response
            if let redis::Value::Array(entries) = pending {
                if let Some(redis::Value::Array(entry)) = entries.first() {
                    if entry.len() >= 3 {
                        let message_id = match &entry[0] {
                            redis::Value::BulkString(bytes) => String::from_utf8_lossy(bytes).to_string(),
                            _ => continue,
                        };

                        let idle_time = match &entry[2] {
                            redis::Value::Int(ms) => *ms as u64,
                            _ => continue,
                        };

                        // Get delivery count (entry[3]) to track actual retry attempts
                        let delivery_count = if entry.len() >= 4 {
                            match &entry[3] {
                                redis::Value::Int(n) => *n as i32,
                                _ => 0,
                            }
                        } else {
                            0
                        };

                        if idle_time > self.config.pending_timeout_ms {
                            // XCLAIM to take ownership
                            let claimed: redis::Value = redis::cmd("XCLAIM")
                                .arg(&stream_key)
                                .arg(&self.config.consumer_group)
                                .arg(worker_id)
                                .arg(self.config.pending_timeout_ms)
                                .arg(&message_id)
                                .query_async(conn)
                                .await?;

                            if let redis::Value::Array(messages) = claimed {
                                if let Some(redis::Value::Array(msg)) = messages.first() {
                                    if msg.len() >= 2 {
                                        let fields = self.parse_stream_fields(&msg[1])?;
                                        tracing::warn!(
                                            delivery_count = delivery_count,
                                            "Claimed pending message {} from dead consumer",
                                            message_id
                                        );
                                        let mut job = self.parse_job(&stream_key, &message_id, &fields)?;
                                        // Use delivery count as actual attempts (Redis tracks this natively)
                                        job.attempts = delivery_count;
                                        return Ok(Some(job));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Parse stream message fields from Redis value.
    fn parse_stream_fields(&self, value: &redis::Value) -> Result<HashMap<String, redis::Value>> {
        let mut fields = HashMap::new();

        if let redis::Value::Array(items) = value {
            let mut iter = items.iter();
            while let (Some(key), Some(val)) = (iter.next(), iter.next()) {
                if let redis::Value::BulkString(key_bytes) = key {
                    let key_str = String::from_utf8_lossy(key_bytes).to_string();
                    fields.insert(key_str, val.clone());
                }
            }
        }

        Ok(fields)
    }

    /// Record job completion.
    pub async fn record_completion(&self, job: &Job, result: JobResult) -> Result<()> {
        self.update_job_status(job.id, JobStatus::Completed, None, Some(&result), None)
            .await
    }

    /// Record job started.
    pub async fn record_started(&self, job: &Job, worker_id: &str) -> Result<()> {
        self.update_job_status(job.id, JobStatus::Running, Some(worker_id), None, None)
            .await
    }
}

#[async_trait]
impl JobQueue for RedisStreamsQueue {
    async fn enqueue(&self, job: Job) -> Result<String> {
        let mut conn = self.get_connection().await?;
        let stream_key = self.config.stream_key(job.job_type.as_str());

        // Serialize payload
        let payload = serde_json::to_string(&job.payload)?;

        // Build XADD command
        let mut cmd = redis::cmd("XADD");
        cmd.arg(&stream_key)
            .arg("MAXLEN")
            .arg("~")
            .arg(self.config.max_stream_length)
            .arg("*") // Auto-generate ID
            .arg("id")
            .arg(job.id.to_string())
            .arg("type")
            .arg(job.job_type.as_str())
            .arg("payload")
            .arg(&payload)
            .arg("priority")
            .arg(job.priority)
            .arg("attempts")
            .arg(job.attempts)
            .arg("max_retries")
            .arg(job.max_retries)
            .arg("created_at")
            .arg(job.created_at.timestamp_millis());

        if let Some(user_id) = job.user_id {
            cmd.arg("user_id").arg(user_id.to_string());
        }

        let message_id: String = cmd.query_async(&mut conn).await?;

        // Record in PostgreSQL history (best-effort — job is already in Redis)
        if let Err(e) = self.record_job_created(&job, &message_id).await {
            tracing::warn!(
                job_id = %job.id,
                error = %e,
                "Failed to record job history in PostgreSQL"
            );
        }

        tracing::info!(
            job_id = %job.id,
            job_type = %job.job_type,
            message_id = %message_id,
            "Job enqueued"
        );

        Ok(message_id)
    }

    async fn dequeue(&self, worker_id: &str) -> Result<Option<Job>> {
        let mut conn = self.get_connection().await?;

        // First, try to claim any pending messages from dead consumers
        if let Some(job) = self.claim_pending_job(&mut conn, worker_id).await? {
            return Ok(Some(job));
        }

        // Build stream keys for all job types
        let streams: Vec<String> = JobType::all()
            .iter()
            .map(|jt| self.config.stream_key(jt.as_str()))
            .collect();

        let stream_refs: Vec<&str> = streams.iter().map(String::as_str).collect();
        let ids: Vec<&str> = vec![">"; streams.len()];

        // Read new messages with XREADGROUP
        let opts = StreamReadOptions::default()
            .group(&self.config.consumer_group, worker_id)
            .block(self.config.block_ms as usize)
            .count(1);

        let result: StreamReadReply = conn
            .xread_options(&stream_refs, &ids, &opts)
            .await?;

        // Parse first message if any
        for stream_key in &result.keys {
            if let Some(message) = stream_key.ids.first() {
                let fields: HashMap<String, redis::Value> = message
                    .map
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                let job = self.parse_job(&stream_key.key, &message.id, &fields)?;
                return Ok(Some(job));
            }
        }

        Ok(None)
    }

    async fn acknowledge(&self, stream_key: &str, message_id: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;

        // XACK to acknowledge processing
        let _: i32 = redis::cmd("XACK")
            .arg(stream_key)
            .arg(&self.config.consumer_group)
            .arg(message_id)
            .query_async(&mut conn)
            .await?;

        // XDEL to remove from stream (keeps stream small)
        let _: i32 = redis::cmd("XDEL")
            .arg(stream_key)
            .arg(message_id)
            .query_async(&mut conn)
            .await?;

        tracing::debug!(
            stream_key = %stream_key,
            message_id = %message_id,
            "Job acknowledged"
        );

        Ok(())
    }

    async fn fail(&self, job: &Job, error: &str) -> Result<()> {
        let stream_key = job.stream_key.as_deref().unwrap_or("");
        let message_id = job.message_id.as_deref().unwrap_or("");
        let mut conn = self.get_connection().await?;
        let dead_letter_key = self.config.dead_letter_key();

        // Serialize the full job payload for the DLQ
        let payload = serde_json::to_string(&job.payload).unwrap_or_default();

        // Add to dead letter stream with full job data
        let mut cmd = redis::cmd("XADD");
        cmd.arg(&dead_letter_key)
            .arg("MAXLEN")
            .arg("~")
            .arg(self.config.max_stream_length)
            .arg("*")
            .arg("id")
            .arg(job.id.to_string())
            .arg("type")
            .arg(job.job_type.as_str())
            .arg("payload")
            .arg(&payload)
            .arg("attempts")
            .arg(job.attempts)
            .arg("max_retries")
            .arg(job.max_retries)
            .arg("original_stream")
            .arg(stream_key)
            .arg("original_id")
            .arg(message_id)
            .arg("error")
            .arg(error)
            .arg("failed_at")
            .arg(Utc::now().timestamp_millis());

        if let Some(user_id) = job.user_id {
            cmd.arg("user_id").arg(user_id.to_string());
        }

        let _: String = cmd.query_async(&mut conn).await?;

        // Acknowledge original message to remove from pending
        self.acknowledge(stream_key, message_id).await?;

        tracing::warn!(
            job_id = %job.id,
            stream_key = %stream_key,
            message_id = %message_id,
            error = %error,
            "Job moved to dead letter queue"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_key_generation() {
        let config = RedisStreamsConfig::default();
        assert_eq!(config.stream_key("download"), "jobs:download");
        assert_eq!(config.dead_letter_key(), "jobs:dead_letter");
    }
}
