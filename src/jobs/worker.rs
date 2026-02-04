//! Worker pool for processing jobs.

use crate::jobs::{Job, JobHandlers, JobQueue, JobStatus, RedisStreamsQueue, WorkerPoolConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::Instrument;

/// Pool of workers processing jobs.
pub struct WorkerPool {
    workers: Vec<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
    config: WorkerPoolConfig,
}

impl WorkerPool {
    /// Start the worker pool.
    pub async fn start(
        config: WorkerPoolConfig,
        queue: Arc<RedisStreamsQueue>,
        handlers: Arc<JobHandlers>,
    ) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let mut workers = Vec::with_capacity(config.num_workers);

        for i in 0..config.num_workers {
            let worker_id = format!("{}-{}", config.worker_name_prefix, i);
            let queue = Arc::clone(&queue);
            let handlers = Arc::clone(&handlers);
            let shutdown = Arc::clone(&shutdown);

            let handle = tokio::spawn(async move {
                let worker = Worker::new(worker_id, queue, handlers, shutdown);
                worker.run().await;
            });

            workers.push(handle);
        }

        tracing::info!(
            num_workers = config.num_workers,
            "Worker pool started"
        );

        Self {
            workers,
            shutdown,
            config,
        }
    }

    /// Initiate graceful shutdown of all workers.
    pub async fn shutdown(self) {
        tracing::info!("Initiating graceful shutdown of worker pool");
        self.shutdown.store(true, Ordering::SeqCst);

        let timeout = Duration::from_secs(self.config.shutdown_timeout_secs);

        // Wait for all workers to complete current jobs
        for (i, handle) in self.workers.into_iter().enumerate() {
            match tokio::time::timeout(timeout, handle).await {
                Ok(Ok(())) => tracing::info!("Worker {} shut down cleanly", i),
                Ok(Err(e)) => tracing::error!("Worker {} panicked: {:?}", i, e),
                Err(_) => tracing::warn!("Worker {} timed out during shutdown", i),
            }
        }

        tracing::info!("Worker pool shutdown complete");
    }

    /// Check if shutdown has been requested.
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Get the number of workers.
    pub fn num_workers(&self) -> usize {
        self.config.num_workers
    }
}

/// A single worker that processes jobs.
pub struct Worker {
    id: String,
    queue: Arc<RedisStreamsQueue>,
    handlers: Arc<JobHandlers>,
    shutdown: Arc<AtomicBool>,
}

impl Worker {
    /// Create a new worker.
    pub fn new(
        id: String,
        queue: Arc<RedisStreamsQueue>,
        handlers: Arc<JobHandlers>,
        shutdown: Arc<AtomicBool>,
    ) -> Self {
        Self {
            id,
            queue,
            handlers,
            shutdown,
        }
    }

    /// Run the worker loop.
    pub async fn run(&self) {
        tracing::info!(worker_id = %self.id, "Worker starting");

        while !self.shutdown.load(Ordering::SeqCst) {
            match self.queue.dequeue(&self.id).await {
                Ok(Some(job)) => {
                    let span = tracing::info_span!(
                        "process_job",
                        job.id = %job.id,
                        job.type = %job.job_type,
                        worker.id = %self.id
                    );

                    self.process_job(job).instrument(span).await;
                }
                Ok(None) => {
                    // No job available, XREADGROUP already blocked
                    continue;
                }
                Err(e) => {
                    tracing::error!(
                        worker_id = %self.id,
                        error = %e,
                        "Failed to dequeue job"
                    );
                    // Back off on error
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }

        tracing::info!(worker_id = %self.id, "Worker stopped");
    }

    /// Process a single job.
    async fn process_job(&self, mut job: Job) {
        let stream_key = job.stream_key.clone().unwrap_or_default();
        let message_id = job.message_id.clone().unwrap_or_default();

        // Get handler for this job type
        let Some(handler) = self.handlers.get(&job.job_type) else {
            tracing::error!(
                job_id = %job.id,
                job_type = %job.job_type,
                "No handler registered for job type"
            );
            if let Err(e) = self
                .queue
                .fail(&job, "No handler registered")
                .await
            {
                tracing::error!(error = %e, "Failed to mark job as failed");
            }
            return;
        };

        // Record job started
        if let Err(e) = self.queue.record_started(&job, &self.id).await {
            tracing::warn!(error = %e, "Failed to record job started");
        }

        job.increment_attempts();

        tracing::info!(
            job_id = %job.id,
            job_type = %job.job_type,
            attempt = job.attempts,
            "Processing job"
        );

        // Execute the job
        match handler.execute(&job).await {
            Ok(result) => {
                // Acknowledge successful completion
                if let Err(e) = self.queue.acknowledge(&stream_key, &message_id).await {
                    tracing::error!(error = %e, "Failed to acknowledge job");
                }

                // Record completion
                if let Err(e) = self.queue.record_completion(&job, result).await {
                    tracing::warn!(error = %e, "Failed to record job completion");
                }

                tracing::info!(
                    job_id = %job.id,
                    job_type = %job.job_type,
                    "Job completed successfully"
                );
            }
            Err(e) if job.can_retry() => {
                // Will be retried via pending message mechanism
                tracing::warn!(
                    job_id = %job.id,
                    job_type = %job.job_type,
                    attempt = job.attempts,
                    max_retries = job.max_retries,
                    error = %e,
                    "Job failed, will retry"
                );
                // Don't acknowledge - let it be reclaimed after pending timeout
            }
            Err(e) => {
                // Max retries exceeded, move to dead letter queue
                if let Err(fail_err) = self
                    .queue
                    .fail(&job, &e.to_string())
                    .await
                {
                    tracing::error!(error = %fail_err, "Failed to move job to dead letter queue");
                }

                // Update job history
                if let Err(history_err) = self
                    .queue
                    .update_job_status(job.id, JobStatus::Failed, Some(&self.id), None, Some(&e.to_string()))
                    .await
                {
                    tracing::warn!(error = %history_err, "Failed to update job history");
                }

                tracing::error!(
                    job_id = %job.id,
                    job_type = %job.job_type,
                    attempts = job.attempts,
                    error = %e,
                    "Job permanently failed"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_pool_config() {
        let config = WorkerPoolConfig::default();
        assert_eq!(config.num_workers, 4);
        assert_eq!(config.shutdown_timeout_secs, 30);
    }
}
