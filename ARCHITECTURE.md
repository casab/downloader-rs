# Architecture Guide

This document describes the architecture of downloader-rs, a file downloading service with user authentication, job queuing, and cloud storage integration.

## Table of Contents

- [Overview](#overview)
- [System Architecture](#system-architecture)
- [Application Layers](#application-layers)
- [Data Flow](#data-flow)
- [Key Components](#key-components)
- [Database Schema](#database-schema)
- [Authentication](#authentication)
- [Job Queue System](#job-queue-system)
- [Observability](#observability)

## Overview

downloader-rs is a Rust-based web application that provides:

- **REST API** for managing file downloads
- **User authentication** with JWT and session support
- **Background job processing** via Redis Streams
- **Multi-provider storage** (S3, GCS, Azure, local)
- **Observability** with OpenTelemetry and Kafka events

### Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust (stable) |
| Web Framework | Actix-web 4.x |
| Database | PostgreSQL 16+ |
| Cache/Queue | Redis 7+ (Streams) |
| Event Bus | Apache Kafka |
| Observability | OpenTelemetry |
| Storage | S3/GCS/Azure/Local |

## System Architecture

```
                                    ┌─────────────────┐
                                    │   Load Balancer │
                                    └────────┬────────┘
                                             │
                    ┌────────────────────────┼────────────────────────┐
                    │                        │                        │
           ┌────────▼────────┐     ┌─────────▼────────┐    ┌─────────▼────────┐
           │  API Server 1   │     │   API Server 2   │    │   API Server N   │
           └────────┬────────┘     └─────────┬────────┘    └─────────┬────────┘
                    │                        │                        │
                    └────────────────────────┼────────────────────────┘
                                             │
        ┌────────────────┬───────────────────┼───────────────────┬────────────────┐
        │                │                   │                   │                │
   ┌────▼────┐    ┌──────▼──────┐    ┌───────▼───────┐    ┌──────▼──────┐  ┌──────▼──────┐
   │PostgreSQL│    │    Redis    │    │    Kafka     │    │   Storage   │  │    OTel     │
   │(Primary) │    │(Queue+Cache)│    │   (Events)   │    │  (S3/etc)   │  │  Collector  │
   └──────────┘    └─────────────┘    └──────────────┘    └─────────────┘  └─────────────┘
                          │
                   ┌──────┴──────┐
                   │   Workers   │
                   │  (1..N)     │
                   └─────────────┘
```

## Application Layers

The application follows a layered architecture:

```
┌─────────────────────────────────────────────────────────────┐
│                        Routes Layer                          │
│   HTTP handlers, request/response parsing, validation        │
├─────────────────────────────────────────────────────────────┤
│                      Middleware Layer                        │
│   Authentication, rate limiting, tracing, error handling     │
├─────────────────────────────────────────────────────────────┤
│                       Service Layer                          │
│   Business logic, orchestration, event emission              │
├─────────────────────────────────────────────────────────────┤
│                      Repository Layer                        │
│   Database access, query construction, data mapping          │
├─────────────────────────────────────────────────────────────┤
│                       Client Layer                           │
│   External services: Storage, Redis, Kafka                   │
└─────────────────────────────────────────────────────────────┘
```

### Directory Structure

```
src/
├── main.rs                 # Entry point
├── lib.rs                  # Library root
├── api.rs                  # Server configuration
├── configuration.rs        # Config loading
├── telemetry.rs           # Observability setup
│
├── routes/                 # HTTP handlers
│   ├── mod.rs
│   ├── auth.rs            # Login, register
│   ├── download.rs        # Download CRUD
│   ├── health_check.rs    # Health endpoints
│   └── user.rs            # User profile
│
├── middlewares/           # Request middleware
│   ├── mod.rs
│   ├── auth.rs            # JWT/Session validation
│   └── rate_limit.rs      # Rate limiting
│
├── repository/            # Data access
│   ├── mod.rs
│   ├── auth.rs            # User queries
│   └── download.rs        # Download queries
│
├── models/                # Domain models
│   ├── mod.rs
│   ├── user.rs
│   ├── download.rs
│   └── pagination.rs
│
├── clients/               # External services
│   ├── mod.rs
│   ├── storage/           # Storage providers
│   └── s3.rs
│
├── jobs/                  # Background jobs
│   ├── mod.rs
│   ├── queue.rs           # Redis Streams queue
│   ├── worker.rs          # Worker pool
│   └── handlers/          # Job handlers
│
├── events/                # Event system
│   ├── mod.rs
│   └── publisher.rs       # Kafka publisher
│
└── utils/                 # Utilities
    ├── mod.rs
    ├── api.rs             # Error helpers
    ├── auth.rs            # Password hashing
    └── file.rs            # File operations
```

## Data Flow

### API Request Flow

```
┌──────────┐     ┌──────────────┐     ┌─────────────┐     ┌──────────────┐
│  Client  │────▶│ NormalizePath│────▶│   Session   │────▶│   Tracing    │
└──────────┘     └──────────────┘     │  Middleware │     │   Logger     │
                                      └─────────────┘     └──────┬───────┘
                                                                 │
    ┌────────────────────────────────────────────────────────────┘
    │
    ▼
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│    Router    │────▶│     Auth     │────▶│   Handler    │
│              │     │  Middleware  │     │              │
└──────────────┘     └──────────────┘     └──────┬───────┘
                                                 │
                     ┌───────────────────────────┼───────────────────────────┐
                     │                           │                           │
              ┌──────▼──────┐            ┌───────▼───────┐           ┌───────▼───────┐
              │ Repository  │            │  Job Queue    │           │    Events     │
              │             │            │               │           │               │
              └──────┬──────┘            └───────┬───────┘           └───────┬───────┘
                     │                           │                           │
              ┌──────▼──────┐            ┌───────▼───────┐           ┌───────▼───────┐
              │  PostgreSQL │            │    Redis      │           │    Kafka      │
              └─────────────┘            └───────────────┘           └───────────────┘
```

### Download Processing Flow

```
┌──────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Client  │────▶│ POST /download│───▶│Create Record │────▶│ Enqueue Job  │
└──────────┘     └──────────────┘     │ (PostgreSQL) │     │   (Redis)    │
                                      └──────────────┘     └──────┬───────┘
                                                                  │
     ┌────────────────────────────────────────────────────────────┘
     │
     ▼
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Worker     │────▶│   Download   │────▶│   Upload to  │────▶│ Mark Complete│
│  (dequeue)   │     │    File      │     │   Storage    │     │   + Event    │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
                            │
                     ┌──────▼──────┐
                     │  Progress   │
                     │  Updates    │
                     │ (Redis PubSub)│
                     └─────────────┘
```

## Key Components

### Routes (`src/routes/`)

HTTP handlers that process requests and return responses.

```rust
// Example: download.rs
pub async fn download(
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
    queue: web::Data<Arc<RedisStreamsQueue>>,
    query: web::Query<DownloadQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    // 1. Create download record
    // 2. Enqueue job
    // 3. Return 202 Accepted
}
```

### Repository (`src/repository/`)

Database access layer using SQLx with compile-time checked queries.

```rust
// Example: download.rs
pub async fn get_download(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> Result<Option<Download>> {
    sqlx::query_as!(
        Download,
        r#"
        SELECT id, url, status as "status: _", ...
        FROM downloads
        WHERE id = $1 AND user_id = $2
        "#,
        id,
        user_id
    )
    .fetch_optional(pool)
    .await
    .context("Failed to fetch download")
}
```

### Job Queue (`src/jobs/`)

Redis Streams-based job queue with consumer groups.

```rust
// Producer
queue.enqueue(Job::new(JobType::Download, payload)).await?;

// Consumer (Worker)
loop {
    if let Some(job) = queue.dequeue(&worker_id).await? {
        let result = handler.execute(&job).await;
        queue.acknowledge(&job.stream_key, &job.message_id).await?;
    }
}
```

### Events (`src/events/`)

Kafka-based event publishing for analytics and integrations.

```rust
// Publish event
let event = Event::new("download.completed", DownloadCompleted {
    download_id,
    url,
    bytes,
});
publisher.publish("downloader.downloads", event).await?;
```

## Database Schema

### Entity Relationship Diagram

```
┌─────────────┐       ┌─────────────────┐       ┌─────────────┐
│   users     │───────│    downloads    │───────│   folders   │
├─────────────┤   1:N ├─────────────────┤   N:1 ├─────────────┤
│ id          │       │ id              │       │ id          │
│ email       │       │ url             │       │ name        │
│ password_hash│      │ status          │       │ path        │
│ is_admin    │       │ file_path       │       │ parent_id   │
│ created_at  │       │ user_id (FK)    │       │ user_id (FK)│
└─────────────┘       │ folder_id (FK)  │       └─────────────┘
       │              │ bytes_downloaded │              │
       │              │ total_bytes      │              │
       │              └─────────────────┘              │
       │                      │                        │
       │              ┌───────▼───────┐                │
       │              │ download_tags │                │
       │              │ (junction)    │                │
       │              ├───────────────┤                │
       │              │ download_id   │                │
       │              │ tag_id        │                │
       │              └───────────────┘                │
       │                      │                        │
       │              ┌───────▼───────┐                │
       │              │     tags      │◀───────────────┘
       │              ├───────────────┤
       │              │ id            │
       │              │ name          │
       │              │ color         │
       │              │ user_id (FK)  │
       │              └───────────────┘
       │
       │    ┌─────────────────┐    ┌─────────────────┐
       └───▶│   job_history   │    │   audit_logs    │
            ├─────────────────┤    ├─────────────────┤
            │ id              │    │ id              │
            │ job_type        │    │ user_id (FK)    │
            │ status          │    │ action          │
            │ payload         │    │ resource_type   │
            │ user_id (FK)    │    │ resource_id     │
            └─────────────────┘    │ old/new_values  │
                                   └─────────────────┘
```

## Authentication

### Dual Authentication Strategy

1. **JWT Token** (Primary)
   - Sent via `Authorization: Bearer <token>` header
   - Contains user_id in `sub` claim
   - Short expiration (7 hours default)

2. **Session** (Fallback)
   - Redis-backed session storage
   - Session ID in secure cookie
   - Used for browser-based clients

```
┌──────────────┐
│   Request    │
└──────┬───────┘
       │
       ▼
┌──────────────┐     Yes    ┌──────────────┐
│ Has Auth     │───────────▶│ Validate JWT │
│ Header?      │            └──────┬───────┘
└──────┬───────┘                   │
       │ No                        │
       ▼                           ▼
┌──────────────┐            ┌──────────────┐
│ Has Session  │───Yes────▶│Get User from │
│ Cookie?      │            │    Redis     │
└──────┬───────┘            └──────┬───────┘
       │ No                        │
       ▼                           ▼
┌──────────────┐            ┌──────────────┐
│ Return 401   │            │ Set user_id  │
│ Unauthorized │            │ in request   │
└──────────────┘            └──────────────┘
```

## Job Queue System

### Redis Streams Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Redis Streams                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   jobs:download:critical  ─────▶  Consumer Group: workers       │
│   jobs:download:high      ─────▶       ├── worker-0             │
│   jobs:download:normal    ─────▶       ├── worker-1             │
│   jobs:download:low       ─────▶       └── worker-N             │
│                                                                  │
│   jobs:email:normal       ─────▶  Consumer Group: workers       │
│   jobs:cleanup:low        ─────▶                                │
│                                                                  │
│   jobs:dead_letter        ◀───── (Failed after max retries)     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Job Lifecycle

```
    PENDING ──────▶ QUEUED ──────▶ RUNNING ──────▶ COMPLETED
       │              │              │
       │              │              │
       │              ▼              ▼
       │          CANCELLED       FAILED
       │                            │
       │                            │ (retry < max)
       │                            ▼
       └────────────────────────▶ QUEUED
                                    │
                                    │ (retry >= max)
                                    ▼
                              DEAD_LETTER
```

## Observability

### Telemetry Pipeline

```
┌──────────────────┐
│   Application    │
│   (tracing)      │
└────────┬─────────┘
         │
         │ OTLP (gRPC)
         ▼
┌──────────────────┐
│  OpenTelemetry   │
│    Collector     │
└────────┬─────────┘
         │
    ┌────┼────┬────────────┐
    │    │    │            │
    ▼    ▼    ▼            ▼
┌──────┐ ┌────────┐ ┌──────────┐ ┌───────┐
│Jaeger│ │Prometheus│ │  Kafka   │ │ Loki  │
│Traces│ │ Metrics │ │ Events  │ │ Logs  │
└──────┘ └────────┘ └──────────┘ └───────┘
```

### Key Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `http_requests_total` | Counter | Total HTTP requests |
| `http_request_duration_seconds` | Histogram | Request latency |
| `downloads_in_progress` | Gauge | Active downloads |
| `downloads_bytes_total` | Counter | Total bytes downloaded |
| `job_queue_depth` | Gauge | Pending jobs |
| `job_processing_duration_seconds` | Histogram | Job execution time |

### Event Types

| Event | Topic | Description |
|-------|-------|-------------|
| `download.started` | downloader.downloads | Download initiated |
| `download.completed` | downloader.downloads | Download finished |
| `download.failed` | downloader.downloads | Download failed |
| `user.registered` | downloader.users | New user signup |
| `user.login` | downloader.users | User authenticated |
| `job.enqueued` | downloader.jobs | Job added to queue |
| `job.completed` | downloader.jobs | Job finished |

## Configuration

### Environment-Based Configuration

```
configuration/
├── base.yaml           # Shared settings
├── local.yaml          # Development overrides
└── production.yaml     # Production overrides
```

Configuration is loaded in order:
1. `base.yaml`
2. `{environment}.yaml`
3. Environment variables (prefix: `APP_`)

### Example Configuration

```yaml
application:
  host: "0.0.0.0"
  port: 8000

database:
  host: "localhost"
  port: 5432
  username: "postgres"
  database_name: "downloader"
  require_ssl: false

redis_uri: "redis://localhost:6379"

queue:
  backend: "redis_streams"
  redis_streams:
    consumer_group: "workers"
    max_retries: 3

telemetry:
  otlp:
    enabled: true
    endpoint: "http://otel-collector:4317"
```

## Security Considerations

1. **Password Storage**: Argon2id with high work factors
2. **JWT Tokens**: Short expiration, validated on each request
3. **SQL Injection**: Prevented by SQLx prepared statements
4. **Secret Management**: `SecretString` for sensitive data
5. **Rate Limiting**: Token bucket algorithm per user/IP
6. **Audit Logging**: All admin actions logged

## Scaling Considerations

### Horizontal Scaling

- **API Servers**: Stateless, scale behind load balancer
- **Workers**: Add more worker instances to Redis consumer group
- **Database**: Read replicas for query scaling

### Bottlenecks

| Component | Scaling Strategy |
|-----------|------------------|
| Database writes | Partitioning, async writes |
| Large downloads | Streaming, chunked processing |
| Job queue | Priority queues, rate limiting |
| Event publishing | Batching, async publishing |
