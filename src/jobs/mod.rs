//! Background job processing with Redis Streams.
//!
//! This module provides a job queue implementation using Redis Streams
//! with consumer groups for reliable, distributed job processing.

mod config;
mod handler;
pub mod handlers;
mod models;
mod queue;
mod worker;

pub use config::{QueueConfig, RedisStreamsConfig, WorkerPoolConfig};
pub use handler::{JobHandler, JobHandlers};
pub use handlers::DownloadJobHandler;
pub use models::{
    CleanupPayload, DownloadPayload, EmailPayload, Job, JobPayload, JobResult, JobStatus, JobType,
    NotificationPayload, WebhookPayload,
};
pub use queue::{JobQueue, RedisStreamsQueue};
pub use worker::{Worker, WorkerPool};
