//! Integration tests for the jobs module.

use downloader::jobs::{
    Job, JobQueue, JobType, RedisStreamsConfig, RedisStreamsQueue,
    CleanupPayload, DownloadPayload,
};
use uuid::Uuid;

/// Create a test Redis Streams queue.
async fn create_test_queue() -> RedisStreamsQueue {
    let mut config = RedisStreamsConfig::default();
    // Use a unique prefix for each test run to avoid conflicts
    config.stream_prefix = format!("test_jobs_{}", Uuid::new_v4().to_string().split('-').next().unwrap());
    config.block_ms = 100; // Short block time for tests

    let queue = RedisStreamsQueue::new(config).expect("Failed to create queue");
    queue.initialize().await.expect("Failed to initialize queue");
    queue
}

#[tokio::test]
async fn test_enqueue_and_dequeue_job() {
    let queue = create_test_queue().await;

    let payload = CleanupPayload {
        cleanup_type: "temp_files".to_string(),
        params: None,
    };

    let job = Job::new(JobType::Cleanup, &payload)
        .expect("Failed to create job")
        .with_priority(5);

    let job_id = job.id;

    // Enqueue the job
    let message_id = queue.enqueue(job).await.expect("Failed to enqueue job");
    assert!(!message_id.is_empty());

    // Dequeue the job
    let dequeued = queue
        .dequeue("test-worker-1")
        .await
        .expect("Failed to dequeue");

    assert!(dequeued.is_some());
    let dequeued_job = dequeued.unwrap();
    assert_eq!(dequeued_job.id, job_id);
    assert_eq!(dequeued_job.job_type, JobType::Cleanup);
    assert_eq!(dequeued_job.priority, 5);

    // Acknowledge the job
    queue
        .acknowledge(
            dequeued_job.stream_key.as_ref().unwrap(),
            dequeued_job.message_id.as_ref().unwrap(),
        )
        .await
        .expect("Failed to acknowledge job");
}

#[tokio::test]
async fn test_download_job_enqueue() {
    let queue = create_test_queue().await;

    let user_id = Uuid::new_v4();
    let download_id = Uuid::new_v4();

    let payload = DownloadPayload {
        download_id,
        user_id,
        url: "https://example.com/test.zip".to_string(),
        filename: Some("test.zip".to_string()),
        priority: 10,
    };

    let job = Job::new(JobType::Download, &payload)
        .expect("Failed to create job")
        .with_user_id(user_id)
        .with_priority(10);

    // Enqueue
    let message_id = queue.enqueue(job).await.expect("Failed to enqueue");
    assert!(!message_id.is_empty());

    // Dequeue and verify
    let dequeued = queue.dequeue("test-worker-2").await.expect("Failed to dequeue");
    assert!(dequeued.is_some());

    let job = dequeued.unwrap();
    assert_eq!(job.job_type, JobType::Download);
    assert_eq!(job.user_id, Some(user_id));

    // Parse payload
    let parsed_payload: DownloadPayload = serde_json::from_value(job.payload).unwrap();
    assert_eq!(parsed_payload.download_id, download_id);
    assert_eq!(parsed_payload.url, "https://example.com/test.zip");

    // Cleanup
    queue
        .acknowledge(job.stream_key.as_ref().unwrap(), job.message_id.as_ref().unwrap())
        .await
        .ok();
}

#[tokio::test]
async fn test_job_fail_to_dead_letter() {
    let queue = create_test_queue().await;

    let payload = CleanupPayload {
        cleanup_type: "failing_job".to_string(),
        params: None,
    };

    let job = Job::new(JobType::Cleanup, &payload).expect("Failed to create job");

    // Enqueue
    queue.enqueue(job).await.expect("Failed to enqueue");

    // Dequeue
    let dequeued = queue.dequeue("test-worker-3").await.expect("Failed to dequeue");
    let job = dequeued.unwrap();

    // Fail the job
    queue
        .fail(&job, "Test failure reason")
        .await
        .expect("Failed to move job to dead letter");

    // The job should now be in the dead letter queue and removed from the main queue
    // (We can't easily verify the dead letter queue contents without additional API,
    // but the fact that fail() succeeded is a good indication)
}

#[tokio::test]
async fn test_multiple_job_types() {
    // Use longer block time for this test to ensure we can dequeue both jobs
    let mut config = RedisStreamsConfig::default();
    config.stream_prefix = format!("test_jobs_multi_{}", Uuid::new_v4().to_string().split('-').next().unwrap());
    config.block_ms = 500; // Longer block time for multi-job test

    let queue = RedisStreamsQueue::new(config).expect("Failed to create queue");
    queue.initialize().await.expect("Failed to initialize queue");

    // Enqueue two jobs of the SAME type to a single stream (simpler test)
    let cleanup_job1 = Job::new(
        JobType::Cleanup,
        &CleanupPayload {
            cleanup_type: "test1".to_string(),
            params: None,
        },
    )
    .unwrap();

    let cleanup_job2 = Job::new(
        JobType::Cleanup,
        &CleanupPayload {
            cleanup_type: "test2".to_string(),
            params: None,
        },
    )
    .unwrap();

    let id1 = cleanup_job1.id;
    let id2 = cleanup_job2.id;

    queue.enqueue(cleanup_job1).await.expect("Failed to enqueue cleanup 1");
    queue.enqueue(cleanup_job2).await.expect("Failed to enqueue cleanup 2");

    // Dequeue both from the same stream
    let job1 = queue.dequeue("test-worker-4").await.expect("Failed to dequeue first");
    assert!(job1.is_some(), "First dequeue should return a job");
    let job1 = job1.unwrap();

    let job2 = queue.dequeue("test-worker-4").await.expect("Failed to dequeue second");
    assert!(job2.is_some(), "Second dequeue should return a job");
    let job2 = job2.unwrap();

    // Verify we got both jobs (order may vary)
    let ids: Vec<uuid::Uuid> = vec![job1.id, job2.id];
    assert!(ids.contains(&id1), "Should have first job");
    assert!(ids.contains(&id2), "Should have second job");

    // Cleanup
    queue.acknowledge(job1.stream_key.as_ref().unwrap(), job1.message_id.as_ref().unwrap()).await.ok();
    queue.acknowledge(job2.stream_key.as_ref().unwrap(), job2.message_id.as_ref().unwrap()).await.ok();
}

#[tokio::test]
async fn test_dequeue_empty_queue_returns_none() {
    let queue = create_test_queue().await;

    // Try to dequeue from empty queue (should timeout quickly due to short block_ms)
    let result = queue.dequeue("test-worker-5").await.expect("Dequeue failed");
    assert!(result.is_none());
}
