# downloader-rs: 16-Week Implementation Plan

## Executive Summary

This document outlines a comprehensive 16-week development roadmap for enhancing downloader-rs with 10 major feature areas. The plan is structured in phases that respect dependencies, maintain code quality through continuous testing and linting, and deliver incremental value.

### Feature Areas Covered
1. CI/CD & Development Infrastructure
2. API Enhancements (Pagination, Filtering, Sorting)
3. User Management (Profile, Password Reset, Email Verification)
4. Download Management (Progress, Pause/Resume, Retry)
5. Background Jobs (Redis Streams Queue System)
6. File Organization (Folders, Tags, Search)
7. Storage Options (Multi-provider Support)
8. Rate Limiting (Per-user Limits, Throttling)
9. Monitoring (OpenTelemetry, Prometheus, Kafka Events)
10. Admin Features (User Management, Audit Logs)
11. Notifications (Webhooks, Email)

---

## Architectural Decisions

This section documents key architectural choices and their rationale.

### ADR-001: Redis Streams for Job Queue

**Decision:** Use Redis Streams as the primary job queue backend instead of PostgreSQL.

**Context:** The system needs a reliable, high-performance job queue for processing downloads asynchronously.

**Rationale:**
| Aspect | Redis Streams | PostgreSQL |
|--------|---------------|------------|
| Latency | Sub-millisecond | 1-10ms |
| Throughput | 100k+ ops/sec | Lower (disk I/O) |
| Queue primitives | Native (XADD, XREADGROUP) | Requires `SKIP LOCKED` |
| Already in stack | Yes (sessions) | Yes |
| Consumer groups | Built-in | Manual implementation |

**Approach:** Hybrid architecture
- **Redis Streams**: Active job queue (fast enqueue/dequeue)
- **PostgreSQL**: Job history, audit trail, analytics

**Consequences:**
- Faster job processing
- Need to handle Redis persistence (RDB/AOF)
- Job history queries go to PostgreSQL

---

### ADR-002: Kafka + OpenTelemetry for Observability

**Decision:** Use OpenTelemetry for telemetry collection with Kafka as an event bus for system events.

**Context:** The system needs comprehensive observability (metrics, traces, logs) and the ability to publish domain events for analytics and integrations.

**Rationale:**
- **OpenTelemetry**: Industry standard, vendor-neutral, unified API for traces/metrics/logs
- **Kafka**: Durable event streaming, replay capability, multiple consumers
- **tracing integration**: Single instrumentation point, emit to multiple backends

**Architecture:**
```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Application   │────▶│  OpenTelemetry   │────▶│  OTel Collector │
│   (tracing)     │     │     Exporter     │     │                 │
└─────────────────┘     └──────────────────┘     └────────┬────────┘
                                                          │
                    ┌─────────────────────────────────────┼─────────────────────────────┐
                    ▼                                     ▼                             ▼
              ┌──────────┐                         ┌──────────────┐              ┌──────────┐
              │  Kafka   │                         │  Prometheus  │              │  Jaeger  │
              │ (events) │                         │  (metrics)   │              │ (traces) │
              └──────────┘                         └──────────────┘              └──────────┘
```

**Event Types Published to Kafka:**
- `download.started`, `download.completed`, `download.failed`
- `user.registered`, `user.login`, `user.password_changed`
- `job.enqueued`, `job.started`, `job.completed`, `job.failed`

**Consequences:**
- Unified telemetry with tracing crate
- Kafka adds operational complexity
- Enables future event-driven features
- Analytics pipelines can consume events

---

## Phase 1: Foundation & Infrastructure (Weeks 1-2)

### Week 1: CI/CD Pipeline & Development Infrastructure

#### Goals
- Establish robust CI/CD pipeline
- Standardize development environment
- Set up comprehensive linting and formatting

#### Tasks

##### Day 1-2: GitHub Actions CI Pipeline
```yaml
# .github/workflows/ci.yml structure:
- Rust toolchain setup (stable + nightly for fmt)
- Caching (cargo registry, target directory)
- Matrix testing (Ubuntu, macOS)
- Parallel jobs: check, fmt, clippy, test
```

| Task | Description | Tests |
|------|-------------|-------|
| Create `.github/workflows/ci.yml` | Main CI workflow | Verify pipeline runs |
| Create `.github/workflows/security.yml` | Security audit with `cargo-audit` | Weekly scheduled runs |
| Add `rust-toolchain.toml` | Pin Rust version for consistency | N/A |
| Configure branch protection | Require CI pass for merges | Manual verification |

##### Day 3-4: Linting & Formatting Standards
| Task | Description | Configuration |
|------|-------------|---------------|
| Configure `rustfmt.toml` | Code formatting rules | `max_width = 100`, `edition = "2021"` |
| Configure `clippy.toml` | Linting rules | Deny warnings, specific lints |
| Add `deny.toml` | Dependency checking | License, security advisories |
| Pre-commit hooks | Local validation | `cargo fmt`, `cargo clippy` |

**rustfmt.toml:**
```toml
max_width = 100
edition = "2021"
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
imports_granularity = "Module"
group_imports = "StdExternalCrate"
```

**clippy configuration in Cargo.toml:**
```toml
[lints.clippy]
pedantic = "warn"
nursery = "warn"
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
```

##### Day 5: Code Coverage Setup
| Task | Description | Target |
|------|-------------|--------|
| Integrate `cargo-tarpaulin` | Coverage reporting | 70% minimum |
| Add coverage to CI | Upload to Codecov | Per-PR reports |
| Create coverage badge | README badge | Visual indicator |

#### Deliverables - Week 1
- [ ] CI pipeline running on all PRs
- [ ] Automated security scanning
- [ ] Code formatting enforced
- [ ] Clippy warnings as errors
- [ ] Coverage reporting active

---

### Week 2: Testing Infrastructure & Documentation

#### Goals
- Enhance testing utilities and patterns
- Establish documentation standards
- Create development environment tooling

#### Tasks

##### Day 1-2: Enhanced Test Infrastructure
| Task | Description | Location |
|------|-------------|----------|
| Create test fixtures module | Reusable test data | `tests/fixtures/` |
| Add factory functions | Generate test entities | `tests/api/helpers.rs` |
| Implement test database pooling | Faster test execution | Connection reuse |
| Add integration test categories | Organize by feature | `#[cfg(feature = "integration")]` |

**New test helper patterns:**
```rust
// tests/fixtures/mod.rs
pub struct DownloadFactory;
impl DownloadFactory {
    pub fn pending() -> DownloadBuilder { ... }
    pub fn completed() -> DownloadBuilder { ... }
    pub fn failed() -> DownloadBuilder { ... }
}

pub struct UserFactory;
impl UserFactory {
    pub fn verified() -> UserBuilder { ... }
    pub fn unverified() -> UserBuilder { ... }
    pub fn admin() -> UserBuilder { ... }
}
```

##### Day 3-4: Documentation Infrastructure
| Task | Description | Output |
|------|-------------|--------|
| Configure `cargo doc` | API documentation | `target/doc/` |
| Add doc comments | Public API docs | All public items |
| Create `CONTRIBUTING.md` | Contribution guidelines | Repository root |
| Create `ARCHITECTURE.md` | System architecture | Detailed diagrams |
| Add OpenAPI/Swagger spec | API documentation | `docs/openapi.yaml` |

##### Day 5: Development Tooling
| Task | Description | Purpose |
|------|-------------|---------|
| Create `Makefile` | Common commands | `make test`, `make lint` |
| Add `scripts/setup.sh` | Dev environment setup | One-command setup |
| Create Docker dev environment | Consistent environment | `docker-compose.dev.yml` |
| Add `sqlx-data.json` generation | Offline compilation | CI without DB |

**Makefile targets:**
```makefile
.PHONY: all test lint fmt check coverage docs

all: fmt lint test

test:
	cargo test --all-features

lint:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --all -- --check

check:
	cargo check --all-targets --all-features

coverage:
	cargo tarpaulin --out Html --output-dir coverage/

docs:
	cargo doc --no-deps --open

db-migrate:
	sqlx migrate run

db-prepare:
	cargo sqlx prepare --workspace
```

#### Deliverables - Week 2
- [ ] Test fixture system
- [ ] API documentation generated
- [ ] CONTRIBUTING.md complete
- [ ] ARCHITECTURE.md with diagrams
- [ ] OpenAPI specification
- [ ] Makefile with all common commands
- [ ] Development setup script

---

## Phase 2: API Enhancements (Weeks 3-4)

### Week 3: Pagination & Filtering Foundation

#### Goals
- Implement generic pagination system
- Add filtering capabilities to list endpoints
- Create reusable query parameter parsing

#### Tasks

##### Day 1-2: Pagination Infrastructure
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `PaginationParams` struct | Query param extraction | Unit tests |
| Create `PaginatedResponse<T>` struct | Standardized response | Unit tests |
| Implement cursor-based pagination | Efficient for large datasets | Integration tests |
| Implement offset-based pagination | Simple use cases | Integration tests |

**New types:**
```rust
// src/models/pagination.rs
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
    pub next_cursor: Option<String>,
}
```

##### Day 3-4: Filtering System
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `FilterParams` trait | Generic filtering | Unit tests |
| Implement `DownloadFilter` | Download-specific filters | Integration tests |
| Add SQL query builder | Dynamic WHERE clauses | Unit tests |
| Validate filter inputs | Prevent SQL injection | Security tests |

**Filter implementation:**
```rust
// src/models/filters.rs
#[derive(Debug, Deserialize)]
pub struct DownloadFilter {
    pub status: Option<DownloadStatus>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub url_contains: Option<String>,
}

impl DownloadFilter {
    pub fn apply_to_query(&self, query: &mut QueryBuilder<Postgres>) {
        // Build WHERE clauses safely
    }
}
```

##### Day 5: Update Existing Endpoints
| Task | Description | Tests Required |
|------|-------------|----------------|
| Update `GET /downloads` | Add pagination + filters | Update existing tests |
| Add response headers | `X-Total-Count`, `Link` | Integration tests |
| Update API documentation | OpenAPI spec updates | Manual verification |

#### Test Requirements - Week 3
```rust
#[tokio::test]
async fn get_downloads_returns_paginated_response() { }

#[tokio::test]
async fn get_downloads_respects_page_size_limit() { }

#[tokio::test]
async fn get_downloads_filters_by_status() { }

#[tokio::test]
async fn get_downloads_filters_by_date_range() { }

#[tokio::test]
async fn pagination_params_validates_max_per_page() { }

#[tokio::test]
async fn cursor_pagination_returns_consistent_results() { }
```

---

### Week 4: Sorting & Advanced Query Features

#### Goals
- Implement sorting capabilities
- Add field selection (sparse fieldsets)
- Create compound query builder

#### Tasks

##### Day 1-2: Sorting Implementation
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `SortParams` struct | Sort field + direction | Unit tests |
| Implement sortable fields whitelist | Security measure | Unit tests |
| Add multi-field sorting | Complex ordering | Integration tests |
| Update repository layer | ORDER BY support | Integration tests |

**Sorting types:**
```rust
// src/models/sorting.rs
#[derive(Debug, Deserialize)]
pub struct SortParams {
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

pub trait Sortable {
    fn allowed_sort_fields() -> &'static [&'static str];
    fn default_sort() -> (&'static str, SortOrder);
}
```

##### Day 3-4: Field Selection
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `FieldSelector` | Parse `?fields=` param | Unit tests |
| Implement sparse fieldsets | Return only requested fields | Integration tests |
| Add field validation | Whitelist allowed fields | Security tests |
| Update serialization | Dynamic field inclusion | Unit tests |

##### Day 5: Query Builder Integration
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create unified `QueryParams` | Combine all params | Unit tests |
| Implement query builder | Compose SQL safely | Integration tests |
| Add query logging | Debug complex queries | Manual verification |
| Performance testing | Ensure index usage | Benchmark tests |

**Unified query parameters:**
```rust
// src/models/query.rs
#[derive(Debug, Deserialize)]
pub struct QueryParams<F: FilterParams> {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[serde(flatten)]
    pub filter: F,
    #[serde(flatten)]
    pub sort: SortParams,
    pub fields: Option<String>,
}
```

#### Deliverables - Week 4
- [ ] Pagination working on all list endpoints
- [ ] Filtering by multiple criteria
- [ ] Sorting by allowed fields
- [ ] Field selection support
- [ ] Updated OpenAPI documentation
- [ ] Performance benchmarks passing
- [ ] 90%+ test coverage on new code

---

## Phase 3: User Management (Weeks 5-6)

### Week 5: User Profile & Account Management

#### Goals
- Implement user profile endpoints
- Add account settings management
- Create profile update workflows

#### Tasks

##### Day 1-2: User Profile Endpoints
| Task | Description | Tests Required |
|------|-------------|----------------|
| `GET /api/v1/me` | Get current user profile | Integration tests |
| `PATCH /api/v1/me` | Update profile fields | Integration tests |
| `DELETE /api/v1/me` | Soft delete account | Integration tests |
| Add profile fields to User model | Display name, avatar URL | Migration + unit tests |

**Database migration:**
```sql
-- migrations/YYYYMMDDHHMMSS_add_user_profile_fields.up.sql
ALTER TABLE users
ADD COLUMN display_name TEXT,
ADD COLUMN avatar_url TEXT,
ADD COLUMN bio TEXT,
ADD COLUMN timezone TEXT DEFAULT 'UTC',
ADD COLUMN deleted_at TIMESTAMP WITH TIME ZONE;

CREATE INDEX idx_users_deleted_at ON users(deleted_at) WHERE deleted_at IS NULL;
```

**New endpoints:**
```rust
// src/routes/user.rs
pub async fn get_current_user(
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error>

pub async fn update_current_user(
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
    body: web::Json<UpdateUserRequest>,
) -> Result<HttpResponse, actix_web::Error>
```

##### Day 3-4: Password Change Flow
| Task | Description | Tests Required |
|------|-------------|----------------|
| `POST /api/v1/me/password` | Change password | Integration tests |
| Verify current password | Security requirement | Security tests |
| Invalidate existing sessions | Security measure | Integration tests |
| Add password history | Prevent reuse | Unit tests |

**Password change request:**
```rust
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: SecretString,
    pub new_password: SecretString,
}
```

##### Day 5: Email Change Flow
| Task | Description | Tests Required |
|------|-------------|----------------|
| `POST /api/v1/me/email` | Request email change | Integration tests |
| Store pending email changes | Database table | Migration |
| Email verification token | Secure token generation | Unit tests |

#### Test Requirements - Week 5
```rust
#[tokio::test]
async fn get_current_user_returns_profile() { }

#[tokio::test]
async fn update_profile_succeeds_with_valid_data() { }

#[tokio::test]
async fn update_profile_rejects_invalid_email() { }

#[tokio::test]
async fn change_password_requires_current_password() { }

#[tokio::test]
async fn change_password_invalidates_other_sessions() { }

#[tokio::test]
async fn delete_account_soft_deletes_user() { }
```

---

### Week 6: Password Reset & Email Verification

#### Goals
- Implement password reset flow
- Add email verification system
- Create token management infrastructure

#### Tasks

##### Day 1-2: Token Infrastructure
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `tokens` table | Store verification tokens | Migration |
| Implement token generation | Secure random tokens | Unit tests |
| Add token expiration | Configurable TTL | Unit tests |
| Create token validation | Check expiry, single use | Integration tests |

**Token table migration:**
```sql
-- migrations/YYYYMMDDHHMMSS_create_tokens_table.up.sql
CREATE TYPE token_type AS ENUM ('password_reset', 'email_verification', 'email_change');

CREATE TABLE tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    token_type token_type NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    used_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    metadata JSONB
);

CREATE INDEX idx_tokens_user_id ON tokens(user_id);
CREATE INDEX idx_tokens_expires_at ON tokens(expires_at) WHERE used_at IS NULL;
```

##### Day 3-4: Password Reset Flow
| Task | Description | Tests Required |
|------|-------------|----------------|
| `POST /api/v1/auth/forgot-password` | Request reset | Integration tests |
| `POST /api/v1/auth/reset-password` | Execute reset | Integration tests |
| Rate limiting on requests | Prevent abuse | Integration tests |
| Email service integration | Send reset emails | Mock tests |

**Password reset endpoints:**
```rust
// src/routes/auth.rs
pub async fn forgot_password(
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    body: web::Json<ForgotPasswordRequest>,
) -> Result<HttpResponse, actix_web::Error>

pub async fn reset_password(
    pool: web::Data<PgPool>,
    body: web::Json<ResetPasswordRequest>,
) -> Result<HttpResponse, actix_web::Error>
```

##### Day 5: Email Verification
| Task | Description | Tests Required |
|------|-------------|----------------|
| Add `email_verified_at` column | Track verification status | Migration |
| `POST /api/v1/auth/verify-email` | Verify email with token | Integration tests |
| `POST /api/v1/auth/resend-verification` | Resend verification | Integration tests |
| Optional verification enforcement | Config flag | Configuration tests |

**User model update:**
```rust
pub struct User {
    // ... existing fields
    pub email_verified_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

#### Deliverables - Week 6
- [ ] Complete password reset flow
- [ ] Email verification system
- [ ] Token management infrastructure
- [ ] Email service abstraction (for future implementation)
- [ ] Rate limiting on auth endpoints
- [ ] Updated API documentation
- [ ] Security review completed

---

## Phase 4: Download Management (Weeks 7-8)

### Week 7: Progress Tracking & Download States

#### Goals
- Implement real-time progress tracking
- Enhance download state machine
- Add download metadata storage

#### Tasks

##### Day 1-2: Enhanced Download Model
| Task | Description | Tests Required |
|------|-------------|----------------|
| Add progress fields | `bytes_downloaded`, `total_bytes` | Migration |
| Add download metadata | `content_type`, `filename` | Migration |
| Implement state machine | Valid state transitions | Unit tests |
| Add error tracking | `error_message`, `retry_count` | Migration |

**Database migration:**
```sql
-- migrations/YYYYMMDDHHMMSS_enhance_downloads_table.up.sql
ALTER TABLE downloads
ADD COLUMN bytes_downloaded BIGINT DEFAULT 0,
ADD COLUMN total_bytes BIGINT,
ADD COLUMN content_type TEXT,
ADD COLUMN filename TEXT,
ADD COLUMN error_message TEXT,
ADD COLUMN retry_count INTEGER DEFAULT 0,
ADD COLUMN max_retries INTEGER DEFAULT 3,
ADD COLUMN priority INTEGER DEFAULT 0,
ADD COLUMN metadata JSONB DEFAULT '{}';

CREATE INDEX idx_downloads_status_priority ON downloads(status, priority DESC);
```

**State machine:**
```rust
// src/models/download.rs
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "download_status", rename_all = "snake_case")]
pub enum DownloadStatus {
    Pending,
    Queued,
    InProgress,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadStatus {
    pub fn can_transition_to(&self, target: &DownloadStatus) -> bool {
        use DownloadStatus::*;
        matches!(
            (self, target),
            (Pending, Queued) |
            (Queued, InProgress) |
            (Queued, Cancelled) |
            (InProgress, Paused) |
            (InProgress, Completed) |
            (InProgress, Failed) |
            (InProgress, Cancelled) |
            (Paused, InProgress) |
            (Paused, Cancelled) |
            (Failed, Queued)  // Retry
        )
    }
}
```

##### Day 3-4: Progress Tracking API
| Task | Description | Tests Required |
|------|-------------|----------------|
| `GET /api/v1/downloads/{id}/progress` | Get progress | Integration tests |
| Progress update mechanism | Internal function | Unit tests |
| Progress percentage calculation | Handle unknown sizes | Unit tests |
| ETA calculation | Based on speed | Unit tests |

**Progress response:**
```rust
#[derive(Debug, Serialize)]
pub struct DownloadProgress {
    pub id: Uuid,
    pub status: DownloadStatus,
    pub bytes_downloaded: i64,
    pub total_bytes: Option<i64>,
    pub percentage: Option<f32>,
    pub speed_bytes_per_sec: Option<i64>,
    pub eta_seconds: Option<i64>,
    pub started_at: Option<DateTime<Utc>>,
    pub elapsed_seconds: Option<i64>,
}
```

##### Day 5: WebSocket Progress Updates (Foundation)
| Task | Description | Tests Required |
|------|-------------|----------------|
| Add actix-web-actors dependency | WebSocket support | N/A |
| Create WebSocket handler | `/ws/downloads/{id}` | Integration tests |
| Design message protocol | Progress update format | Documentation |

#### Test Requirements - Week 7
```rust
#[tokio::test]
async fn download_state_transitions_are_validated() { }

#[tokio::test]
async fn get_progress_returns_current_state() { }

#[tokio::test]
async fn progress_percentage_handles_unknown_size() { }

#[tokio::test]
async fn eta_calculation_is_accurate() { }
```

---

### Week 8: Pause/Resume & Retry Logic

#### Goals
- Implement pause/resume functionality
- Add intelligent retry logic
- Create download control endpoints

#### Tasks

##### Day 1-2: Pause/Resume Implementation
| Task | Description | Tests Required |
|------|-------------|----------------|
| `POST /api/v1/downloads/{id}/pause` | Pause download | Integration tests |
| `POST /api/v1/downloads/{id}/resume` | Resume download | Integration tests |
| HTTP Range request support | Resume from byte offset | Unit tests |
| Store resume position | Persist progress | Integration tests |

**Control endpoints:**
```rust
// src/routes/download.rs
pub async fn pause_download(
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, actix_web::Error>

pub async fn resume_download(
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
    download_manager: web::Data<DownloadManager>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, actix_web::Error>
```

##### Day 3-4: Retry Logic
| Task | Description | Tests Required |
|------|-------------|----------------|
| `POST /api/v1/downloads/{id}/retry` | Manual retry | Integration tests |
| Automatic retry on failure | Configurable attempts | Integration tests |
| Exponential backoff | Prevent hammering | Unit tests |
| Retry policy configuration | Per-download settings | Configuration tests |

**Retry configuration:**
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_base: f64,
}

impl RetryPolicy {
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay = self.initial_delay_ms as f64
            * self.exponential_base.powi(attempt as i32);
        Duration::from_millis(delay.min(self.max_delay_ms as f64) as u64)
    }
}
```

##### Day 5: Download Cancellation & Cleanup
| Task | Description | Tests Required |
|------|-------------|----------------|
| `POST /api/v1/downloads/{id}/cancel` | Cancel download | Integration tests |
| Clean up partial files | Remove incomplete downloads | Integration tests |
| Cancel in-flight requests | Abort HTTP connection | Unit tests |
| Batch cancellation | Cancel multiple downloads | Integration tests |

#### Deliverables - Week 8
- [ ] Pause/resume working for HTTP Range-supported servers
- [ ] Intelligent retry with exponential backoff
- [ ] Download cancellation with cleanup
- [ ] Progress tracking API complete
- [ ] WebSocket foundation ready
- [ ] Comprehensive test coverage

---

## Phase 5: Background Jobs with Redis Streams (Weeks 9-10)

### Week 9: Redis Streams Job Queue Infrastructure

#### Goals
- Implement Redis Streams-based job queue
- Create worker pool with consumer groups
- Add PostgreSQL for job history/audit

#### Tasks

##### Day 1-2: Redis Streams Setup
| Task | Description | Tests Required |
|------|-------------|----------------|
| Add `redis` crate with streams support | Cargo.toml update | N/A |
| Create Redis connection pool | Shared Redis client | Unit tests |
| Implement stream initialization | Create streams on startup | Integration tests |
| Create consumer group | `XGROUP CREATE` on startup | Integration tests |

**Dependencies:**
```toml
# Cargo.toml
[dependencies]
redis = { version = "0.25", features = ["tokio-comp", "streams"] }
```

**Redis Streams configuration:**
```yaml
# configuration/base.yaml
queue:
  backend: "redis_streams"

  redis_streams:
    url: "redis://localhost:6379"
    stream_prefix: "jobs"           # jobs:download, jobs:email, etc.
    consumer_group: "workers"
    consumer_name_prefix: "worker"
    block_ms: 5000                  # Block timeout for XREADGROUP
    max_retries: 3
    retry_delay_ms: 5000
    pending_timeout_ms: 300000      # 5 minutes - reclaim dead consumer jobs

  # PostgreSQL for job history
  history:
    enabled: true
    retention_days: 90
```

**Stream initialization:**
```rust
// src/jobs/redis_queue.rs
pub struct RedisStreamsQueue {
    client: redis::Client,
    config: RedisStreamsConfig,
}

impl RedisStreamsQueue {
    pub async fn initialize(&self) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        // Create streams and consumer groups for each job type
        for job_type in JobType::all() {
            let stream_key = format!("{}:{}", self.config.stream_prefix, job_type);

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
                Ok(_) => tracing::info!("Created consumer group for {}", stream_key),
                Err(e) if e.to_string().contains("BUSYGROUP") => {
                    tracing::debug!("Consumer group already exists for {}", stream_key);
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
}
```

##### Day 3-4: Job Queue Implementation
| Task | Description | Tests Required |
|------|-------------|----------------|
| Implement `enqueue` with XADD | Add jobs to stream | Unit tests |
| Implement `dequeue` with XREADGROUP | Consumer group reading | Integration tests |
| Implement `acknowledge` with XACK | Mark job complete | Integration tests |
| Implement pending message recovery | XPENDING + XCLAIM | Integration tests |

**Redis Streams job queue trait implementation:**
```rust
// src/jobs/redis_queue.rs
use redis::streams::{StreamReadOptions, StreamReadReply};

#[async_trait]
impl JobQueue for RedisStreamsQueue {
    async fn enqueue(&self, job: Job) -> Result<String> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let stream_key = format!("{}:{}", self.config.stream_prefix, job.job_type);

        // Serialize job to fields
        let payload = serde_json::to_string(&job.payload)?;

        // XADD with auto-generated ID
        let message_id: String = redis::cmd("XADD")
            .arg(&stream_key)
            .arg("*")  // Auto-generate ID
            .arg("id").arg(job.id.to_string())
            .arg("type").arg(job.job_type.as_str())
            .arg("payload").arg(&payload)
            .arg("priority").arg(job.priority)
            .arg("created_at").arg(Utc::now().timestamp_millis())
            .query_async(&mut conn)
            .await?;

        // Also record in PostgreSQL for history
        if self.config.history.enabled {
            self.record_job_history(&job, &message_id).await?;
        }

        Ok(message_id)
    }

    async fn dequeue(&self, worker_id: &str) -> Result<Option<Job>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        // First, try to claim any pending messages from dead consumers
        if let Some(job) = self.claim_pending_job(&mut conn, worker_id).await? {
            return Ok(Some(job));
        }

        // Read new messages with XREADGROUP
        let streams: Vec<String> = JobType::all()
            .iter()
            .map(|jt| format!("{}:{}", self.config.stream_prefix, jt))
            .collect();

        let opts = StreamReadOptions::default()
            .group(&self.config.consumer_group, worker_id)
            .block(self.config.block_ms)
            .count(1);

        let result: StreamReadReply = conn.xread_options(&streams, &[">"; streams.len()], &opts).await?;

        // Parse first message if any
        if let Some(stream_key) = result.keys.first() {
            if let Some(message) = stream_key.ids.first() {
                return Ok(Some(self.parse_job(message)?));
            }
        }

        Ok(None)
    }

    async fn acknowledge(&self, stream_key: &str, message_id: &str) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        // XACK to acknowledge processing
        redis::cmd("XACK")
            .arg(stream_key)
            .arg(&self.config.consumer_group)
            .arg(message_id)
            .query_async(&mut conn)
            .await?;

        // XDEL to remove from stream (optional, keeps stream small)
        redis::cmd("XDEL")
            .arg(stream_key)
            .arg(message_id)
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    async fn fail(&self, stream_key: &str, message_id: &str, error: &str) -> Result<()> {
        // Move to dead letter stream after max retries
        let mut conn = self.client.get_multiplexed_async_connection().await?;

        let dead_letter_key = format!("{}:dead_letter", self.config.stream_prefix);

        // Add to dead letter stream with error info
        redis::cmd("XADD")
            .arg(&dead_letter_key)
            .arg("*")
            .arg("original_stream").arg(stream_key)
            .arg("original_id").arg(message_id)
            .arg("error").arg(error)
            .arg("failed_at").arg(Utc::now().timestamp_millis())
            .query_async(&mut conn)
            .await?;

        // Acknowledge original message
        self.acknowledge(stream_key, message_id).await?;

        Ok(())
    }
}

impl RedisStreamsQueue {
    /// Claim pending messages from consumers that have been idle too long
    async fn claim_pending_job(
        &self,
        conn: &mut redis::aio::MultiplexedConnection,
        worker_id: &str,
    ) -> Result<Option<Job>> {
        for job_type in JobType::all() {
            let stream_key = format!("{}:{}", self.config.stream_prefix, job_type);

            // XPENDING to find stuck messages
            let pending: Vec<(String, String, i64, i64)> = redis::cmd("XPENDING")
                .arg(&stream_key)
                .arg(&self.config.consumer_group)
                .arg("-")
                .arg("+")
                .arg(1)
                .query_async(conn)
                .await?;

            if let Some((message_id, _consumer, idle_time, _delivery_count)) = pending.first() {
                if *idle_time > self.config.pending_timeout_ms as i64 {
                    // XCLAIM to take ownership
                    let claimed: Vec<StreamId> = redis::cmd("XCLAIM")
                        .arg(&stream_key)
                        .arg(&self.config.consumer_group)
                        .arg(worker_id)
                        .arg(self.config.pending_timeout_ms)
                        .arg(message_id)
                        .query_async(conn)
                        .await?;

                    if let Some(message) = claimed.first() {
                        tracing::warn!(
                            "Claimed pending message {} from dead consumer",
                            message_id
                        );
                        return Ok(Some(self.parse_job(message)?));
                    }
                }
            }
        }
        Ok(None)
    }
}
```

##### Day 5: Job History in PostgreSQL
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `job_history` table | Audit and analytics | Migration |
| Record job lifecycle events | Start, complete, fail | Integration tests |
| Query endpoints for history | Filter, paginate | Integration tests |

**Job history migration (PostgreSQL):**
```sql
-- migrations/YYYYMMDDHHMMSS_create_job_history_table.up.sql
CREATE TYPE job_status AS ENUM ('pending', 'running', 'completed', 'failed', 'cancelled');
CREATE TYPE job_type AS ENUM ('download', 'email', 'cleanup', 'notification', 'webhook');

CREATE TABLE job_history (
    id UUID PRIMARY KEY,
    stream_message_id TEXT,         -- Redis stream message ID
    job_type job_type NOT NULL,
    status job_status NOT NULL,
    payload JSONB NOT NULL,
    result JSONB,
    error_message TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    worker_id TEXT,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_job_history_user_id ON job_history(user_id);
CREATE INDEX idx_job_history_status ON job_history(status);
CREATE INDEX idx_job_history_type ON job_history(job_type);
CREATE INDEX idx_job_history_created_at ON job_history(created_at DESC);

-- Partition by month for efficient cleanup
-- (Consider using pg_partman for automatic partition management)
```

#### Test Requirements - Week 9
```rust
#[tokio::test]
async fn redis_streams_enqueue_adds_to_stream() { }

#[tokio::test]
async fn consumer_group_distributes_jobs() { }

#[tokio::test]
async fn xack_removes_from_pending() { }

#[tokio::test]
async fn dead_consumer_jobs_are_claimed() { }

#[tokio::test]
async fn failed_jobs_go_to_dead_letter() { }

#[tokio::test]
async fn job_history_records_lifecycle() { }
```

---

### Week 10: Worker Pool & Download Integration

#### Goals
- Implement robust worker pool with consumer groups
- Integrate downloads with Redis Streams queue
- Add real-time progress via Redis Pub/Sub

#### Tasks

##### Day 1-2: Worker Pool with Consumer Groups
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `WorkerPool` manager | Spawn/manage workers | Unit tests |
| Implement worker with XREADGROUP | Block on multiple streams | Integration tests |
| Add graceful shutdown | SIGTERM handling | Integration tests |
| Worker health monitoring | Heartbeat mechanism | Integration tests |

**Worker pool implementation:**
```rust
// src/jobs/worker.rs
pub struct WorkerPool {
    workers: Vec<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
    config: WorkerPoolConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkerPoolConfig {
    pub num_workers: usize,
    pub worker_name_prefix: String,
}

impl WorkerPool {
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

        Self { workers, shutdown, config }
    }

    pub async fn shutdown(self) {
        tracing::info!("Initiating graceful shutdown of worker pool");
        self.shutdown.store(true, Ordering::SeqCst);

        // Wait for all workers to complete current jobs
        for (i, handle) in self.workers.into_iter().enumerate() {
            match tokio::time::timeout(Duration::from_secs(30), handle).await {
                Ok(Ok(())) => tracing::info!("Worker {} shut down cleanly", i),
                Ok(Err(e)) => tracing::error!("Worker {} panicked: {:?}", i, e),
                Err(_) => tracing::warn!("Worker {} timed out during shutdown", i),
            }
        }
    }
}

pub struct Worker {
    id: String,
    queue: Arc<RedisStreamsQueue>,
    handlers: Arc<JobHandlers>,
    shutdown: Arc<AtomicBool>,
}

impl Worker {
    pub async fn run(&self) {
        tracing::info!("Worker {} starting", self.id);

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
                    tracing::error!("Failed to dequeue job: {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }

        tracing::info!("Worker {} stopped", self.id);
    }

    async fn process_job(&self, job: Job) {
        let handler = self.handlers.get(&job.job_type);

        match handler.execute(&job).await {
            Ok(result) => {
                self.queue.acknowledge(&job.stream_key, &job.message_id).await.ok();
                self.queue.record_completion(&job, result).await.ok();
            }
            Err(e) if job.attempts < job.max_retries => {
                // Will be retried via pending message mechanism
                tracing::warn!("Job {} failed (attempt {}): {}", job.id, job.attempts, e);
            }
            Err(e) => {
                self.queue.fail(&job.stream_key, &job.message_id, &e.to_string()).await.ok();
                tracing::error!("Job {} permanently failed: {}", job.id, e);
            }
        }
    }
}
```

##### Day 3-4: Download Queue Integration
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `DownloadJobHandler` | Process download jobs | Integration tests |
| Update download endpoint | Enqueue to Redis Streams | Integration tests |
| Progress updates via Pub/Sub | Real-time progress | Integration tests |
| Link downloads to jobs | `job_message_id` column | Migration |

**Download handler:**
```rust
// src/jobs/handlers/download.rs
pub struct DownloadJobHandler {
    pool: PgPool,
    storage: Arc<dyn StorageProvider>,
    redis: redis::Client,
}

#[async_trait]
impl JobHandler for DownloadJobHandler {
    async fn execute(&self, job: &Job) -> Result<JobResult> {
        let payload: DownloadPayload = serde_json::from_value(job.payload.clone())?;

        // Update download status to InProgress
        repository::update_download_status(
            &self.pool,
            payload.download_id,
            DownloadStatus::InProgress,
        ).await?;

        // Perform download with progress reporting
        let result = self.download_with_progress(&payload).await;

        match &result {
            Ok(download_result) => {
                repository::complete_download(
                    &self.pool,
                    payload.download_id,
                    &download_result.file_path,
                    download_result.total_bytes,
                ).await?;

                // Publish completion event
                self.publish_event(DownloadEvent::Completed {
                    download_id: payload.download_id,
                    user_id: payload.user_id,
                    file_path: download_result.file_path.clone(),
                    bytes: download_result.total_bytes,
                }).await?;
            }
            Err(e) => {
                repository::fail_download(
                    &self.pool,
                    payload.download_id,
                    &e.to_string(),
                ).await?;

                // Publish failure event
                self.publish_event(DownloadEvent::Failed {
                    download_id: payload.download_id,
                    user_id: payload.user_id,
                    error: e.to_string(),
                }).await?;
            }
        }

        result.map(|r| JobResult::Download(r))
    }
}

impl DownloadJobHandler {
    async fn download_with_progress(&self, payload: &DownloadPayload) -> Result<DownloadResult> {
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        let progress_channel = format!("progress:{}", payload.download_id);

        // Download with progress callback
        let progress_callback = |bytes_downloaded: u64, total_bytes: Option<u64>| {
            let progress = DownloadProgress {
                download_id: payload.download_id,
                bytes_downloaded,
                total_bytes,
                percentage: total_bytes.map(|t| (bytes_downloaded as f32 / t as f32) * 100.0),
            };

            // Publish progress via Redis Pub/Sub (fire and forget)
            let _ = redis::cmd("PUBLISH")
                .arg(&progress_channel)
                .arg(serde_json::to_string(&progress).unwrap())
                .query_async::<()>(&mut conn);
        };

        // Actual download implementation...
        download_file_with_progress(&payload.url, progress_callback).await
    }
}
```

**Updated download endpoint:**
```rust
// src/routes/download.rs
pub async fn download(
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
    queue: web::Data<Arc<RedisStreamsQueue>>,
    query: web::Query<DownloadQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    // Create download record
    let download = repository::create_download(&pool, &user_id.0, &query.url).await.map_err(e500)?;

    // Create job
    let job = Job::new(
        JobType::Download,
        serde_json::to_value(DownloadPayload {
            download_id: download.id,
            url: query.url.clone(),
            user_id: user_id.0,
        }).map_err(e500)?,
    );

    // Enqueue to Redis Streams
    let message_id = queue.enqueue(job).await.map_err(e500)?;

    // Link job to download
    repository::set_download_job_id(&pool, download.id, &message_id).await.map_err(e500)?;

    // Return 202 Accepted with download info
    Ok(HttpResponse::Accepted().json(DownloadResponse {
        id: download.id,
        status: DownloadStatus::Queued,
        job_message_id: message_id,
        url: query.url.clone(),
    }))
}
```

##### Day 5: Job Monitoring & Priority Queues
| Task | Description | Tests Required |
|------|-------------|----------------|
| Priority-based stream selection | High priority first | Integration tests |
| `GET /api/v1/jobs` | List user's jobs from history | Integration tests |
| Queue depth metrics | XLEN, XPENDING counts | Metrics |
| Dead letter queue inspection | Admin endpoint | Integration tests |

**Priority queue implementation:**
```rust
// Priority is handled by having separate streams per priority level
// Workers check high-priority streams first

impl RedisStreamsQueue {
    pub async fn enqueue_with_priority(&self, job: Job, priority: JobPriority) -> Result<String> {
        let stream_key = format!(
            "{}:{}:{}",
            self.config.stream_prefix,
            job.job_type,
            priority.as_str()  // "critical", "high", "normal", "low"
        );

        // ... XADD to priority-specific stream
    }
}

impl Worker {
    async fn dequeue_by_priority(&self) -> Result<Option<Job>> {
        // Check streams in priority order
        for priority in [JobPriority::Critical, JobPriority::High, JobPriority::Normal, JobPriority::Low] {
            let streams = self.get_streams_for_priority(priority);
            if let Some(job) = self.queue.read_from_streams(&streams, &self.id).await? {
                return Ok(Some(job));
            }
        }
        Ok(None)
    }
}
```

#### Deliverables - Week 10
- [ ] Redis Streams job queue fully operational
- [ ] Worker pool with consumer groups
- [ ] Downloads processed via job queue
- [ ] Real-time progress via Redis Pub/Sub
- [ ] Job history in PostgreSQL
- [ ] Priority-based processing
- [ ] Dead letter queue handling
- [ ] Comprehensive test coverage

---

## Phase 6: File Organization (Weeks 11-12)

### Week 11: Folders & Tags System

#### Goals
- Implement folder hierarchy for downloads
- Add tagging system
- Create organization management endpoints

#### Tasks

##### Day 1-2: Folders Implementation
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `folders` table | Hierarchical storage | Migration |
| CRUD endpoints for folders | Create, read, update, delete | Integration tests |
| Move downloads to folders | Update download.folder_id | Integration tests |
| Nested folder support | Parent-child relationships | Integration tests |

**Folders migration:**
```sql
-- migrations/YYYYMMDDHHMMSS_create_folders_table.up.sql
CREATE TABLE folders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES folders(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    path TEXT NOT NULL,  -- Materialized path: /parent/child/grandchild
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, path)
);

CREATE INDEX idx_folders_user_id ON folders(user_id);
CREATE INDEX idx_folders_parent_id ON folders(parent_id);
CREATE INDEX idx_folders_path ON folders(path);

ALTER TABLE downloads ADD COLUMN folder_id UUID REFERENCES folders(id) ON DELETE SET NULL;
CREATE INDEX idx_downloads_folder_id ON downloads(folder_id);
```

**Folder endpoints:**
```
POST   /api/v1/folders              Create folder
GET    /api/v1/folders              List root folders
GET    /api/v1/folders/{id}         Get folder with contents
PATCH  /api/v1/folders/{id}         Update folder (rename, move)
DELETE /api/v1/folders/{id}         Delete folder
POST   /api/v1/folders/{id}/move    Move folder to new parent
```

##### Day 3-4: Tags Implementation
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `tags` table | Tag definitions | Migration |
| Create `download_tags` junction | Many-to-many | Migration |
| Tag CRUD endpoints | Manage tags | Integration tests |
| Add/remove tags from downloads | Tag operations | Integration tests |

**Tags migration:**
```sql
-- migrations/YYYYMMDDHHMMSS_create_tags_tables.up.sql
CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT,  -- Hex color for UI
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, name)
);

CREATE TABLE download_tags (
    download_id UUID NOT NULL REFERENCES downloads(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (download_id, tag_id)
);

CREATE INDEX idx_tags_user_id ON tags(user_id);
CREATE INDEX idx_download_tags_tag_id ON download_tags(tag_id);
```

**Tag endpoints:**
```
POST   /api/v1/tags                     Create tag
GET    /api/v1/tags                     List user's tags
PATCH  /api/v1/tags/{id}                Update tag
DELETE /api/v1/tags/{id}                Delete tag
POST   /api/v1/downloads/{id}/tags      Add tags to download
DELETE /api/v1/downloads/{id}/tags/{tag_id}  Remove tag from download
```

##### Day 5: Bulk Operations
| Task | Description | Tests Required |
|------|-------------|----------------|
| Bulk move downloads | Move multiple to folder | Integration tests |
| Bulk tag operations | Add/remove tags in bulk | Integration tests |
| Bulk delete | Delete multiple downloads | Integration tests |

**Bulk operation endpoints:**
```
POST /api/v1/downloads/bulk/move    { "download_ids": [...], "folder_id": "..." }
POST /api/v1/downloads/bulk/tag     { "download_ids": [...], "tag_ids": [...] }
POST /api/v1/downloads/bulk/delete  { "download_ids": [...] }
```

#### Test Requirements - Week 11
```rust
#[tokio::test]
async fn create_folder_succeeds() { }

#[tokio::test]
async fn nested_folders_maintain_path() { }

#[tokio::test]
async fn move_folder_updates_children_paths() { }

#[tokio::test]
async fn delete_folder_cascades_to_children() { }

#[tokio::test]
async fn tags_are_unique_per_user() { }

#[tokio::test]
async fn bulk_move_updates_all_downloads() { }
```

---

### Week 12: Search Functionality

#### Goals
- Implement full-text search for downloads
- Add advanced search filters
- Create search result ranking

#### Tasks

##### Day 1-2: Full-Text Search Setup
| Task | Description | Tests Required |
|------|-------------|----------------|
| Add PostgreSQL full-text search | tsvector column | Migration |
| Create search index | GIN index on tsvector | Migration |
| Implement search query parser | Parse user queries | Unit tests |
| Basic search endpoint | `GET /api/v1/search` | Integration tests |

**Search migration:**
```sql
-- migrations/YYYYMMDDHHMMSS_add_search_capabilities.up.sql
ALTER TABLE downloads
ADD COLUMN search_vector tsvector
GENERATED ALWAYS AS (
    setweight(to_tsvector('english', coalesce(filename, '')), 'A') ||
    setweight(to_tsvector('english', coalesce(url, '')), 'B')
) STORED;

CREATE INDEX idx_downloads_search ON downloads USING GIN(search_vector);

-- Add search vector for folders
ALTER TABLE folders
ADD COLUMN search_vector tsvector
GENERATED ALWAYS AS (to_tsvector('english', name)) STORED;

CREATE INDEX idx_folders_search ON folders USING GIN(search_vector);
```

##### Day 3-4: Advanced Search Features
| Task | Description | Tests Required |
|------|-------------|----------------|
| Search by file type | Filter by content_type | Integration tests |
| Search within folders | Scope to folder tree | Integration tests |
| Search by tags | Filter by tag names | Integration tests |
| Date range search | Created/completed dates | Integration tests |
| Size range search | File size filters | Integration tests |

**Search request:**
```rust
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,                           // Search query
    pub folder_id: Option<Uuid>,             // Scope to folder
    pub tags: Option<Vec<String>>,           // Filter by tags
    pub status: Option<Vec<DownloadStatus>>, // Filter by status
    pub content_type: Option<String>,        // Filter by MIME type
    pub min_size: Option<i64>,               // Minimum file size
    pub max_size: Option<i64>,               // Maximum file size
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}
```

##### Day 5: Search Ranking & Suggestions
| Task | Description | Tests Required |
|------|-------------|----------------|
| Search result ranking | Relevance scoring | Unit tests |
| Search suggestions | Autocomplete endpoint | Integration tests |
| Recent searches | Store search history | Integration tests |
| Popular searches | Aggregate user searches | Analytics |

**Search response:**
```rust
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: i64,
    pub took_ms: i64,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub id: Uuid,
    pub result_type: SearchResultType,  // Download or Folder
    pub title: String,
    pub snippet: Option<String>,  // Highlighted match
    pub score: f32,
    pub data: serde_json::Value,  // Full entity data
}
```

#### Deliverables - Week 12
- [ ] Folder hierarchy system
- [ ] Tagging system with colors
- [ ] Bulk operations for organization
- [ ] Full-text search
- [ ] Advanced search filters
- [ ] Search suggestions
- [ ] Comprehensive test coverage

---

## Phase 7: Storage & Rate Limiting (Weeks 13-14)

### Week 13: Multi-Provider Storage

#### Goals
- Abstract storage layer
- Add multiple cloud provider support
- Implement storage management

#### Tasks

##### Day 1-2: Storage Abstraction
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `StorageProvider` trait | Common interface | Unit tests |
| Refactor S3 client | Implement trait | Integration tests |
| Add local filesystem provider | Development/backup | Integration tests |
| Storage configuration | Select provider in config | Configuration tests |

**Storage trait:**
```rust
// src/clients/storage/mod.rs
#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String>;
    async fn download(&self, key: &str) -> Result<Vec<u8>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn get_url(&self, key: &str, expires_in: Duration) -> Result<String>;
    async fn list(&self, prefix: &str) -> Result<Vec<StorageObject>>;
    async fn get_metadata(&self, key: &str) -> Result<StorageMetadata>;
}

#[derive(Debug)]
pub struct StorageObject {
    pub key: String,
    pub size: i64,
    pub last_modified: DateTime<Utc>,
    pub content_type: Option<String>,
}
```

##### Day 3-4: Additional Providers
| Task | Description | Tests Required |
|------|-------------|----------------|
| Google Cloud Storage provider | GCS support | Integration tests |
| Azure Blob Storage provider | Azure support | Integration tests |
| MinIO compatibility | S3-compatible testing | Integration tests |
| Provider factory | Create from config | Unit tests |

**Storage configuration:**
```yaml
# configuration/base.yaml
storage:
  provider: "s3"  # s3, gcs, azure, local

  s3:
    bucket: "downloads"
    region: "us-east-1"
    endpoint: null  # Custom endpoint for MinIO
    access_key: "..."
    secret_key: "..."

  gcs:
    bucket: "downloads"
    project_id: "my-project"
    credentials_file: "/path/to/credentials.json"

  azure:
    container: "downloads"
    account_name: "..."
    account_key: "..."

  local:
    base_path: "/var/downloads"
```

##### Day 5: Storage Management
| Task | Description | Tests Required |
|------|-------------|----------------|
| Storage quota tracking | Per-user limits | Integration tests |
| Storage cleanup job | Remove orphaned files | Integration tests |
| Storage statistics | Usage reporting | Integration tests |
| File deduplication | Content-addressable storage | Unit tests |

**Storage quota:**
```rust
#[derive(Debug, Serialize)]
pub struct StorageQuota {
    pub used_bytes: i64,
    pub quota_bytes: i64,
    pub file_count: i64,
    pub max_file_count: Option<i64>,
}
```

#### Test Requirements - Week 13
```rust
#[tokio::test]
async fn s3_provider_uploads_and_downloads() { }

#[tokio::test]
async fn local_provider_stores_files() { }

#[tokio::test]
async fn storage_factory_creates_correct_provider() { }

#[tokio::test]
async fn quota_is_enforced_on_upload() { }

#[tokio::test]
async fn cleanup_job_removes_orphaned_files() { }
```

---

### Week 14: Rate Limiting & Throttling

#### Goals
- Implement API rate limiting
- Add bandwidth throttling for downloads
- Create usage tracking system

#### Tasks

##### Day 1-2: API Rate Limiting
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create rate limiter middleware | Request counting | Unit tests |
| Redis-backed rate limiting | Distributed counting | Integration tests |
| Per-endpoint limits | Different limits per route | Configuration tests |
| Rate limit headers | Standard headers in response | Integration tests |

**Rate limiter middleware:**
```rust
// src/middlewares/rate_limit.rs
pub struct RateLimiter {
    redis: Arc<redis::Client>,
    config: RateLimitConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub burst_size: u32,
}

impl RateLimiter {
    pub async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError> {
        // Token bucket algorithm implementation
    }
}

#[derive(Debug)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: u32,
    pub reset_at: DateTime<Utc>,
    pub retry_after: Option<Duration>,
}
```

**Response headers:**
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1640000000
Retry-After: 60  (only when rate limited)
```

##### Day 3-4: Download Bandwidth Throttling
| Task | Description | Tests Required |
|------|-------------|----------------|
| Per-download speed limit | Configurable per download | Unit tests |
| Per-user bandwidth limit | Aggregate user bandwidth | Integration tests |
| Global bandwidth limit | System-wide throttling | Integration tests |
| Dynamic throttling | Adjust based on load | Integration tests |

**Throttled reader:**
```rust
// src/utils/throttle.rs
pub struct ThrottledReader<R> {
    inner: R,
    rate_limiter: RateLimiter,
    bytes_per_second: u64,
}

impl<R: AsyncRead + Unpin> AsyncRead for ThrottledReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // Implement token bucket for bandwidth
    }
}
```

##### Day 5: Usage Tracking & Limits
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `usage` table | Track usage metrics | Migration |
| Concurrent download limits | Per-user limits | Integration tests |
| Daily/monthly limits | Period-based limits | Integration tests |
| Usage reporting endpoint | `GET /api/v1/me/usage` | Integration tests |

**Usage tracking:**
```sql
-- migrations/YYYYMMDDHHMMSS_create_usage_table.up.sql
CREATE TABLE usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    period_start DATE NOT NULL,
    period_type TEXT NOT NULL,  -- 'daily', 'monthly'
    downloads_count INTEGER NOT NULL DEFAULT 0,
    bytes_downloaded BIGINT NOT NULL DEFAULT 0,
    api_requests INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, period_start, period_type)
);

CREATE INDEX idx_usage_user_period ON usage(user_id, period_start);
```

#### Deliverables - Week 14
- [ ] Multi-provider storage support
- [ ] Storage quota system
- [ ] API rate limiting
- [ ] Download bandwidth throttling
- [ ] Usage tracking and reporting
- [ ] Comprehensive test coverage

---

## Phase 8: Observability with OpenTelemetry & Kafka (Weeks 15-16)

### Week 15: OpenTelemetry Integration & Kafka Events

#### Goals
- Implement OpenTelemetry for unified observability
- Set up Kafka for event streaming
- Create tracing-based event emission

#### Tasks

##### Day 1-2: OpenTelemetry Setup
| Task | Description | Tests Required |
|------|-------------|----------------|
| Add OpenTelemetry dependencies | Cargo.toml update | N/A |
| Configure OTLP exporter | Export to collector | Integration tests |
| Integrate with tracing | tracing-opentelemetry | Unit tests |
| Add trace context propagation | HTTP headers | Integration tests |

**Dependencies:**
```toml
# Cargo.toml
[dependencies]
opentelemetry = { version = "0.22", features = ["metrics", "trace"] }
opentelemetry_sdk = { version = "0.22", features = ["rt-tokio"] }
opentelemetry-otlp = { version = "0.15", features = ["grpc-tonic", "metrics"] }
opentelemetry-semantic-conventions = "0.14"
tracing-opentelemetry = "0.23"

# Kafka
rdkafka = { version = "0.36", features = ["cmake-build", "ssl"] }
```

**OpenTelemetry configuration:**
```yaml
# configuration/base.yaml
telemetry:
  service_name: "downloader-rs"
  service_version: "1.0.0"

  otlp:
    enabled: true
    endpoint: "http://otel-collector:4317"
    protocol: "grpc"  # or "http"

  tracing:
    enabled: true
    sample_rate: 1.0  # 100% in dev, lower in prod

  metrics:
    enabled: true
    export_interval_seconds: 60

  logging:
    format: "json"
    level: "info"
```

**OpenTelemetry initialization:**
```rust
// src/telemetry.rs
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{BatchConfig, RandomIdGenerator, Sampler, Tracer},
    Resource,
};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_telemetry(config: &TelemetryConfig) -> Result<()> {
    // Create resource with service info
    let resource = Resource::new(vec![
        KeyValue::new(SERVICE_NAME, config.service_name.clone()),
        KeyValue::new(SERVICE_VERSION, config.service_version.clone()),
    ]);

    // Configure OTLP exporter
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&config.otlp.endpoint);

    // Create tracer provider
    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(Sampler::TraceIdRatioBased(config.tracing.sample_rate))
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource.clone()),
        )
        .with_batch_config(BatchConfig::default())
        .install_batch(runtime::Tokio)?;

    let tracer = tracer_provider.tracer("downloader-rs");

    // Create metrics provider
    let meter_provider = opentelemetry_otlp::new_pipeline()
        .metrics(runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.otlp.endpoint),
        )
        .with_resource(resource)
        .with_period(Duration::from_secs(config.metrics.export_interval_seconds))
        .build()?;

    // Set global providers
    opentelemetry::global::set_tracer_provider(tracer_provider);
    opentelemetry::global::set_meter_provider(meter_provider);

    // Build tracing subscriber with OpenTelemetry layer
    let otel_layer = OpenTelemetryLayer::new(tracer);

    let subscriber = tracing_subscriber::registry()
        .with(otel_layer)
        .with(tracing_subscriber::fmt::layer().json())
        .with(tracing_subscriber::EnvFilter::from_default_env());

    subscriber.init();

    Ok(())
}

pub fn shutdown_telemetry() {
    opentelemetry::global::shutdown_tracer_provider();
}
```

##### Day 3-4: Kafka Event Producer
| Task | Description | Tests Required |
|------|-------------|----------------|
| Set up Kafka producer | rdkafka configuration | Integration tests |
| Create event schema | Structured event types | Unit tests |
| Implement event publisher | Async event emission | Integration tests |
| Add tracing integration | Emit events from spans | Unit tests |

**Kafka configuration:**
```yaml
# configuration/base.yaml
events:
  enabled: true

  kafka:
    brokers: ["localhost:9092"]
    client_id: "downloader-rs"
    topic_prefix: "downloader"

    producer:
      acks: "all"
      retries: 3
      linger_ms: 5
      batch_size: 16384

    # Topics created automatically
    topics:
      downloads: "downloader.downloads"
      users: "downloader.users"
      jobs: "downloader.jobs"
      system: "downloader.system"
```

**Event types and publisher:**
```rust
// src/events/mod.rs
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub metadata: EventMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventMetadata {
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub user_id: Option<Uuid>,
    pub correlation_id: Option<String>,
}

impl Event {
    pub fn new<T: Serialize>(event_type: &str, data: T) -> Self {
        // Extract trace context from current span
        let span = tracing::Span::current();
        let trace_id = span.context().span().span_context().trace_id().to_string();
        let span_id = span.context().span().span_context().span_id().to_string();

        Self {
            id: Uuid::new_v4(),
            event_type: event_type.to_string(),
            source: "downloader-rs".to_string(),
            timestamp: Utc::now(),
            data: serde_json::to_value(data).unwrap_or_default(),
            metadata: EventMetadata {
                trace_id: Some(trace_id),
                span_id: Some(span_id),
                user_id: None,
                correlation_id: None,
            },
        }
    }

    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.metadata.user_id = Some(user_id);
        self
    }
}

// Event types
pub mod events {
    use super::*;

    // Download events
    #[derive(Debug, Serialize)]
    pub struct DownloadStarted {
        pub download_id: Uuid,
        pub url: String,
        pub user_id: Uuid,
    }

    #[derive(Debug, Serialize)]
    pub struct DownloadCompleted {
        pub download_id: Uuid,
        pub url: String,
        pub user_id: Uuid,
        pub file_path: String,
        pub bytes: i64,
        pub duration_ms: i64,
    }

    #[derive(Debug, Serialize)]
    pub struct DownloadFailed {
        pub download_id: Uuid,
        pub url: String,
        pub user_id: Uuid,
        pub error: String,
        pub attempts: i32,
    }

    // User events
    #[derive(Debug, Serialize)]
    pub struct UserRegistered {
        pub user_id: Uuid,
        pub email: String,
    }

    #[derive(Debug, Serialize)]
    pub struct UserLoggedIn {
        pub user_id: Uuid,
        pub method: String,  // "password", "token"
    }

    // Job events
    #[derive(Debug, Serialize)]
    pub struct JobEnqueued {
        pub job_id: Uuid,
        pub job_type: String,
        pub priority: i32,
    }

    #[derive(Debug, Serialize)]
    pub struct JobCompleted {
        pub job_id: Uuid,
        pub job_type: String,
        pub duration_ms: i64,
    }
}

// Kafka publisher
pub struct EventPublisher {
    producer: FutureProducer,
    config: EventsConfig,
}

impl EventPublisher {
    pub fn new(config: &EventsConfig) -> Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", config.kafka.brokers.join(","))
            .set("client.id", &config.kafka.client_id)
            .set("acks", &config.kafka.producer.acks)
            .set("retries", config.kafka.producer.retries.to_string())
            .set("linger.ms", config.kafka.producer.linger_ms.to_string())
            .set("batch.size", config.kafka.producer.batch_size.to_string())
            .create()?;

        Ok(Self { producer, config: config.clone() })
    }

    #[tracing::instrument(skip(self, event), fields(event_type = %event.event_type))]
    pub async fn publish(&self, topic: &str, event: Event) -> Result<()> {
        let key = event.id.to_string();
        let payload = serde_json::to_string(&event)?;

        let record = FutureRecord::to(topic)
            .key(&key)
            .payload(&payload);

        self.producer
            .send(record, Duration::from_secs(5))
            .await
            .map_err(|(e, _)| anyhow::anyhow!("Kafka send error: {}", e))?;

        tracing::debug!(event_id = %event.id, "Event published to Kafka");
        Ok(())
    }

    pub async fn publish_download_event(&self, event: impl Serialize, event_type: &str, user_id: Uuid) -> Result<()> {
        let event = Event::new(event_type, event).with_user(user_id);
        self.publish(&self.config.kafka.topics.downloads, event).await
    }
}
```

##### Day 5: Tracing Layer for Automatic Event Emission
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create custom tracing layer | Intercept spans | Unit tests |
| Auto-emit events from spans | `emit_event = true` | Integration tests |
| Configure event filtering | Which spans emit | Configuration tests |

**Custom tracing layer for Kafka events:**
```rust
// src/telemetry/kafka_layer.rs
use tracing::{Event, Subscriber, span};
use tracing_subscriber::{layer::Context, Layer};

pub struct KafkaEventLayer {
    publisher: Arc<EventPublisher>,
    event_patterns: Vec<String>,  // Span names that should emit events
}

impl KafkaEventLayer {
    pub fn new(publisher: Arc<EventPublisher>, patterns: Vec<String>) -> Self {
        Self { publisher, event_patterns: patterns }
    }
}

impl<S: Subscriber> Layer<S> for KafkaEventLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Check for emit_event field
        let mut should_emit = false;
        let mut event_type = String::new();
        let mut event_data = serde_json::Map::new();

        event.record(&mut |field: &tracing::field::Field, value: &dyn std::fmt::Debug| {
            match field.name() {
                "emit_event" => should_emit = true,
                "event_type" => event_type = format!("{:?}", value),
                _ => {
                    event_data.insert(
                        field.name().to_string(),
                        serde_json::Value::String(format!("{:?}", value)),
                    );
                }
            }
        });

        if should_emit && !event_type.is_empty() {
            let kafka_event = crate::events::Event::new(
                &event_type,
                serde_json::Value::Object(event_data),
            );

            let publisher = Arc::clone(&self.publisher);
            let topic = self.topic_for_event_type(&event_type);

            // Fire and forget - don't block the span
            tokio::spawn(async move {
                if let Err(e) = publisher.publish(&topic, kafka_event).await {
                    tracing::warn!("Failed to publish event: {}", e);
                }
            });
        }
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        // Optionally emit events when spans close
        if let Some(span) = ctx.span(&id) {
            let name = span.name();
            if self.event_patterns.iter().any(|p| name.contains(p)) {
                // Emit span completion event
            }
        }
    }
}
```

**Usage in application code:**
```rust
// Automatic event emission via tracing
#[tracing::instrument(fields(emit_event = true, event_type = "download.started"))]
pub async fn start_download(download_id: Uuid, url: &str, user_id: Uuid) {
    tracing::info!(
        download_id = %download_id,
        url = %url,
        user_id = %user_id,
        "Download started"
    );
    // The KafkaEventLayer will automatically emit this as a Kafka event
}
```

#### Test Requirements - Week 15
```rust
#[tokio::test]
async fn opentelemetry_exports_traces() { }

#[tokio::test]
async fn kafka_publisher_sends_events() { }

#[tokio::test]
async fn tracing_layer_emits_to_kafka() { }

#[tokio::test]
async fn trace_context_propagated_to_events() { }

#[tokio::test]
async fn metrics_exported_to_otlp() { }
```

---

### Week 16: Metrics, Health Checks & Admin Features

#### Goals
- Implement Prometheus-compatible metrics via OpenTelemetry
- Enhance health checks
- Build admin features and audit logging

#### Tasks

##### Day 1-2: Metrics via OpenTelemetry
| Task | Description | Tests Required |
|------|-------------|----------------|
| Define custom metrics | Counters, histograms, gauges | Unit tests |
| Add HTTP metrics middleware | Request duration, status | Integration tests |
| Add business metrics | Downloads, jobs, users | Integration tests |
| Prometheus endpoint | `/metrics` for scraping | Integration tests |

**Metrics implementation:**
```rust
// src/metrics.rs
use opentelemetry::{
    global,
    metrics::{Counter, Histogram, Meter, UpDownCounter},
    KeyValue,
};

pub struct Metrics {
    pub http_requests_total: Counter<u64>,
    pub http_request_duration: Histogram<f64>,
    pub downloads_total: Counter<u64>,
    pub downloads_in_progress: UpDownCounter<i64>,
    pub downloads_bytes_total: Counter<u64>,
    pub job_queue_depth: UpDownCounter<i64>,
    pub job_processing_duration: Histogram<f64>,
    pub active_users: UpDownCounter<i64>,
}

impl Metrics {
    pub fn new() -> Self {
        let meter = global::meter("downloader-rs");

        Self {
            http_requests_total: meter
                .u64_counter("http_requests_total")
                .with_description("Total number of HTTP requests")
                .init(),

            http_request_duration: meter
                .f64_histogram("http_request_duration_seconds")
                .with_description("HTTP request duration in seconds")
                .init(),

            downloads_total: meter
                .u64_counter("downloads_total")
                .with_description("Total number of downloads initiated")
                .init(),

            downloads_in_progress: meter
                .i64_up_down_counter("downloads_in_progress")
                .with_description("Number of downloads currently in progress")
                .init(),

            downloads_bytes_total: meter
                .u64_counter("downloads_bytes_total")
                .with_description("Total bytes downloaded")
                .init(),

            job_queue_depth: meter
                .i64_up_down_counter("job_queue_depth")
                .with_description("Number of jobs waiting in queue")
                .init(),

            job_processing_duration: meter
                .f64_histogram("job_processing_duration_seconds")
                .with_description("Job processing duration in seconds")
                .init(),

            active_users: meter
                .i64_up_down_counter("active_users")
                .with_description("Number of currently active users")
                .init(),
        }
    }
}

// Metrics middleware for Actix
pub struct MetricsMiddleware {
    metrics: Arc<Metrics>,
}

impl<S, B> Transform<S, ServiceRequest> for MetricsMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    // ... middleware implementation that records metrics
}
```

**Prometheus exposition endpoint:**
```rust
// For Prometheus scraping, use prometheus-client with OTLP
// Or expose via OpenTelemetry Collector's Prometheus exporter

pub async fn metrics_handler() -> impl Responder {
    // Export metrics in Prometheus format
    // This works when using the Prometheus exporter in OTel Collector
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(prometheus_client::encoding::text::encode_to_string(&REGISTRY).unwrap())
}
```

##### Day 3-4: Enhanced Health Checks
| Task | Description | Tests Required |
|------|-------------|----------------|
| Detailed health endpoint | `/api/v1/health` | Integration tests |
| Database health check | Connection + query test | Integration tests |
| Redis health check | PING + queue depth | Integration tests |
| Kafka health check | Producer connectivity | Integration tests |
| Storage health check | Access test | Integration tests |

**Health check implementation:**
```rust
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HashMap<String, ComponentHealth>,
}

#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

pub async fn health_check(
    pool: web::Data<PgPool>,
    redis: web::Data<redis::Client>,
    kafka: web::Data<EventPublisher>,
    storage: web::Data<Arc<dyn StorageProvider>>,
    start_time: web::Data<Instant>,
) -> impl Responder {
    let mut checks = HashMap::new();

    // Database check
    let db_check = check_database(&pool).await;
    let db_healthy = db_check.status == HealthStatus::Healthy;
    checks.insert("database".to_string(), db_check);

    // Redis check
    let redis_check = check_redis(&redis).await;
    let redis_healthy = redis_check.status == HealthStatus::Healthy;
    checks.insert("redis".to_string(), redis_check);

    // Kafka check
    let kafka_check = check_kafka(&kafka).await;
    let kafka_healthy = kafka_check.status == HealthStatus::Healthy;
    checks.insert("kafka".to_string(), kafka_check);

    // Storage check
    let storage_check = check_storage(&storage).await;
    let storage_healthy = storage_check.status == HealthStatus::Healthy;
    checks.insert("storage".to_string(), storage_check);

    // Overall status
    let status = if db_healthy && redis_healthy && kafka_healthy && storage_healthy {
        HealthStatus::Healthy
    } else if db_healthy && redis_healthy {
        HealthStatus::Degraded
    } else {
        HealthStatus::Unhealthy
    };

    let response = HealthResponse {
        status: status.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: start_time.elapsed().as_secs(),
        checks,
    };

    match status {
        HealthStatus::Healthy => HttpResponse::Ok().json(response),
        HealthStatus::Degraded => HttpResponse::Ok().json(response),
        HealthStatus::Unhealthy => HttpResponse::ServiceUnavailable().json(response),
    }
}

async fn check_database(pool: &PgPool) -> ComponentHealth {
    let start = Instant::now();
    match sqlx::query("SELECT 1").fetch_one(pool).await {
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

async fn check_redis(client: &redis::Client) -> ComponentHealth {
    let start = Instant::now();
    match client.get_multiplexed_async_connection().await {
        Ok(mut conn) => {
            let pong: RedisResult<String> = redis::cmd("PING").query_async(&mut conn).await;
            match pong {
                Ok(_) => {
                    // Also check queue depths
                    let queue_info = get_queue_info(&mut conn).await.ok();
                    ComponentHealth {
                        status: HealthStatus::Healthy,
                        latency_ms: Some(start.elapsed().as_millis() as u64),
                        message: None,
                        details: queue_info.map(|i| serde_json::to_value(i).unwrap()),
                    }
                }
                Err(e) => ComponentHealth {
                    status: HealthStatus::Unhealthy,
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    message: Some(e.to_string()),
                    details: None,
                },
            }
        }
        Err(e) => ComponentHealth {
            status: HealthStatus::Unhealthy,
            latency_ms: None,
            message: Some(e.to_string()),
            details: None,
        },
    }
}
```

##### Day 5: Admin Features & Audit Logging
| Task | Description | Tests Required |
|------|-------------|----------------|
| Add admin role to users | `is_admin` column | Migration |
| Admin middleware | Verify admin status | Unit tests |
| Audit log table | Track all actions | Migration |
| Admin endpoints | User management, stats | Integration tests |

**Admin migration:**
```sql
-- migrations/YYYYMMDDHHMMSS_add_admin_role.up.sql
ALTER TABLE users ADD COLUMN is_admin BOOLEAN NOT NULL DEFAULT FALSE;
CREATE INDEX idx_users_is_admin ON users(is_admin) WHERE is_admin = TRUE;
```

**Audit log migration:**
```sql
-- migrations/YYYYMMDDHHMMSS_create_audit_logs_table.up.sql
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id UUID,
    old_values JSONB,
    new_values JSONB,
    ip_address INET,
    user_agent TEXT,
    trace_id TEXT,  -- Link to distributed trace
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at DESC);
CREATE INDEX idx_audit_logs_trace_id ON audit_logs(trace_id);
```

**Admin endpoints:**
```rust
// Admin routes - all require admin middleware
pub fn admin_routes() -> Scope {
    web::scope("/admin")
        .wrap(from_fn(require_admin))
        .route("/stats", web::get().to(get_admin_stats))
        .route("/users", web::get().to(list_users))
        .route("/users/{id}", web::get().to(get_user))
        .route("/users/{id}", web::patch().to(update_user))
        .route("/users/{id}", web::delete().to(delete_user))
        .route("/downloads", web::get().to(list_all_downloads))
        .route("/jobs", web::get().to(list_all_jobs))
        .route("/jobs/dead-letter", web::get().to(list_dead_letter_jobs))
        .route("/audit-logs", web::get().to(list_audit_logs))
        .route("/events", web::get().to(list_recent_events))
}

#[derive(Debug, Serialize)]
pub struct AdminStats {
    pub users: UserStats,
    pub downloads: DownloadStats,
    pub jobs: JobStats,
    pub storage: StorageStats,
    pub events: EventStats,
}

#[derive(Debug, Serialize)]
pub struct EventStats {
    pub events_today: i64,
    pub events_this_hour: i64,
    pub kafka_lag: Option<i64>,
}
```

#### Deliverables - Week 16
- [ ] OpenTelemetry fully integrated (traces, metrics, logs)
- [ ] Kafka event streaming operational
- [ ] Prometheus-compatible metrics endpoint
- [ ] Comprehensive health checks (DB, Redis, Kafka, Storage)
- [ ] Admin role and middleware
- [ ] Audit logging with trace correlation
- [ ] Admin dashboard endpoints
- [ ] Complete documentation

---

### Week 16: Admin Features & Audit Logging

#### Goals
- Implement admin user management
- Add comprehensive audit logging
- Create admin dashboard endpoints

#### Tasks

##### Day 1-2: Admin Role & Endpoints
| Task | Description | Tests Required |
|------|-------------|----------------|
| Add admin role to users | `is_admin` column | Migration |
| Admin middleware | Verify admin status | Unit tests |
| `GET /api/v1/admin/users` | List all users | Integration tests |
| `GET /api/v1/admin/users/{id}` | Get user details | Integration tests |
| `PATCH /api/v1/admin/users/{id}` | Update user | Integration tests |
| `DELETE /api/v1/admin/users/{id}` | Delete user | Integration tests |

**Admin migration:**
```sql
-- migrations/YYYYMMDDHHMMSS_add_admin_role.up.sql
ALTER TABLE users ADD COLUMN is_admin BOOLEAN NOT NULL DEFAULT FALSE;
CREATE INDEX idx_users_is_admin ON users(is_admin) WHERE is_admin = TRUE;
```

**Admin middleware:**
```rust
// src/middlewares/admin.rs
pub async fn require_admin(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let user_id = req.extensions().get::<UserId>()
        .ok_or_else(|| e401(anyhow!("Not authenticated")))?;

    let pool = req.app_data::<web::Data<PgPool>>()
        .ok_or_else(|| e500(anyhow!("Database pool not found")))?;

    let is_admin = repository::is_admin(&pool, user_id.0)
        .await
        .map_err(e500)?;

    if !is_admin {
        return Err(e403(anyhow!("Admin access required")));
    }

    next.call(req).await
}
```

##### Day 3-4: Audit Logging
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `audit_logs` table | Action tracking | Migration |
| Audit log middleware | Capture all actions | Integration tests |
| Admin audit log endpoint | `GET /api/v1/admin/audit-logs` | Integration tests |
| Audit log retention | Cleanup old logs | Scheduled job |

**Audit log migration:**
```sql
-- migrations/YYYYMMDDHHMMSS_create_audit_logs_table.up.sql
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id UUID,
    old_values JSONB,
    new_values JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at);
```

**Audit log struct:**
```rust
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,         // "create", "update", "delete", "login", etc.
    pub resource_type: String,  // "user", "download", "folder", etc.
    pub resource_id: Option<Uuid>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

##### Day 5: Admin Dashboard & Statistics
| Task | Description | Tests Required |
|------|-------------|----------------|
| `GET /api/v1/admin/stats` | System statistics | Integration tests |
| `GET /api/v1/admin/downloads` | All downloads | Integration tests |
| `GET /api/v1/admin/jobs` | Job queue status | Integration tests |
| Admin impersonation | Act as user | Integration tests |

**Admin statistics response:**
```rust
#[derive(Debug, Serialize)]
pub struct AdminStats {
    pub users: UserStats,
    pub downloads: DownloadStats,
    pub storage: StorageStats,
    pub jobs: JobStats,
}

#[derive(Debug, Serialize)]
pub struct UserStats {
    pub total: i64,
    pub active_today: i64,
    pub active_this_week: i64,
    pub new_this_month: i64,
}

#[derive(Debug, Serialize)]
pub struct DownloadStats {
    pub total: i64,
    pub completed: i64,
    pub failed: i64,
    pub in_progress: i64,
    pub total_bytes: i64,
}
```

#### Deliverables - Week 16
- [ ] Prometheus metrics exposed
- [ ] Comprehensive health checks
- [ ] Admin role and middleware
- [ ] User management endpoints
- [ ] Audit logging system
- [ ] Admin dashboard endpoints
- [ ] Complete documentation

---

## Notifications System (Integrated Throughout)

### Email Notifications (Week 6 onwards)
| Notification | Trigger | Template |
|--------------|---------|----------|
| Welcome email | User registration | `welcome.html` |
| Email verification | Registration, email change | `verify_email.html` |
| Password reset | Forgot password request | `password_reset.html` |
| Download complete | Large download finished | `download_complete.html` |
| Download failed | After all retries exhausted | `download_failed.html` |

### Webhook System (Week 10 onwards)
| Task | Description | Week |
|------|-------------|------|
| Webhook endpoint registration | User configures webhooks | 10 |
| Webhook delivery system | Reliable delivery with retry | 10 |
| Webhook events | download.completed, download.failed, etc. | 10-16 |
| Webhook signatures | HMAC signatures for security | 10 |

**Webhook configuration:**
```rust
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Webhook {
    pub id: Uuid,
    pub user_id: Uuid,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
```

**Webhook payload:**
```json
{
    "event": "download.completed",
    "timestamp": "2024-01-15T10:30:00Z",
    "data": {
        "download_id": "...",
        "url": "...",
        "file_path": "...",
        "size_bytes": 1234567
    }
}
```

---

## Testing Strategy Summary

### Test Coverage Targets
| Phase | Unit Tests | Integration Tests | Coverage Target |
|-------|------------|-------------------|-----------------|
| Phase 1 (Weeks 1-2) | CI/CD setup | Pipeline validation | 70% baseline |
| Phase 2 (Weeks 3-4) | Pagination logic | API endpoints | 80% |
| Phase 3 (Weeks 5-6) | Token handling | Auth flows | 85% |
| Phase 4 (Weeks 7-8) | State machine | Download operations | 85% |
| Phase 5 (Weeks 9-10) | Job queue logic | Worker operations | 80% |
| Phase 6 (Weeks 11-12) | Search parsing | Organization endpoints | 80% |
| Phase 7 (Weeks 13-14) | Throttle logic | Storage operations | 80% |
| Phase 8 (Weeks 15-16) | Metrics collection | Admin endpoints | 75% |

### Test Categories
```bash
# Unit tests (fast, no external dependencies)
cargo test --lib

# Integration tests (require database, Redis)
cargo test --test '*'

# Specific feature tests
cargo test --features "integration" download_

# Redis Streams tests
cargo test --features "integration" redis_queue_

# Kafka event tests (requires Kafka running)
cargo test --features "kafka" events_

# Performance/load tests
cargo test --release --features "benchmark"

# Full integration with all services
docker compose -f docker-compose.test.yml up -d
cargo test --features "full-integration"
```

### Continuous Testing
- Pre-commit: `cargo fmt --check && cargo clippy`
- PR checks: Full test suite + coverage
- Nightly: Security audit + dependency check
- Weekly: Performance regression tests

---

## Risk Mitigation

### Technical Risks
| Risk | Mitigation | Week |
|------|------------|------|
| Database migration failures | Reversible migrations, backups | All |
| Breaking API changes | API versioning, deprecation policy | All |
| Performance degradation | Benchmarks, load testing | Weekly |
| Security vulnerabilities | Audit, dependency scanning | Weekly |

### Schedule Risks
| Risk | Mitigation |
|------|------------|
| Feature creep | Strict scope per week |
| Integration issues | Early integration testing |
| External dependencies | Mock services for testing |

---

## Success Metrics

### Per-Phase Metrics
| Phase | Key Metrics |
|-------|-------------|
| Foundation | CI green rate > 95%, coverage > 70% |
| API Enhancements | Response time < 100ms (p95) |
| User Management | Auth flow completion > 99% |
| Download Management | Resume success rate > 90% |
| Background Jobs (Redis Streams) | Job completion rate > 99%, queue latency < 10ms |
| File Organization | Search latency < 200ms (p95) |
| Storage | Upload success rate > 99.9% |
| Observability (OTel + Kafka) | Trace sampling > 99%, event delivery > 99.9% |

### Final Success Criteria
- [ ] All 10 feature areas implemented
- [ ] Test coverage > 80%
- [ ] API documentation complete
- [ ] No critical security vulnerabilities
- [ ] Performance benchmarks met
- [ ] Admin features operational
- [ ] Redis Streams job queue operational with < 10ms latency
- [ ] Kafka event streaming with full trace correlation
- [ ] OpenTelemetry traces visible in Jaeger
- [ ] Prometheus metrics scraped and visualized in Grafana

---

## Appendix: Weekly Checklist Template

```markdown
## Week N Checklist

### Planning
- [ ] Review previous week's deliverables
- [ ] Confirm week's scope
- [ ] Update project board

### Development
- [ ] Implement features per daily breakdown
- [ ] Write unit tests alongside code
- [ ] Create integration tests
- [ ] Update API documentation

### Quality
- [ ] Run full test suite
- [ ] Check code coverage
- [ ] Run clippy and fix warnings
- [ ] Security review for new endpoints

### Documentation
- [ ] Update CHANGELOG.md
- [ ] Update API documentation
- [ ] Add inline documentation

### Review
- [ ] Code review completed
- [ ] PR merged to development branch
- [ ] Demo to stakeholders (if applicable)
```

---

---

## Appendix B: Infrastructure Setup

### Development Environment (docker-compose.dev.yml)

```yaml
version: "3.8"

services:
  # Application
  downloader-rs:
    build:
      context: .
      dockerfile: Dockerfile.dev
    ports:
      - "8000:8000"
    environment:
      - APP_ENVIRONMENT=local
      - DATABASE_URL=postgres://postgres:password@postgres:5432/downloader
      - REDIS_URL=redis://redis:6379
      - KAFKA_BROKERS=kafka:9092
      - OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
    depends_on:
      - postgres
      - redis
      - kafka
      - otel-collector
    volumes:
      - ./:/app
      - cargo-cache:/usr/local/cargo/registry

  # PostgreSQL
  postgres:
    image: postgres:16-alpine
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
      POSTGRES_DB: downloader
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5

  # Redis (Sessions + Job Queue)
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    command: redis-server --appendonly yes
    volumes:
      - redis-data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 5

  # Kafka (Event Streaming)
  zookeeper:
    image: confluentinc/cp-zookeeper:7.5.0
    environment:
      ZOOKEEPER_CLIENT_PORT: 2181
      ZOOKEEPER_TICK_TIME: 2000
    volumes:
      - zookeeper-data:/var/lib/zookeeper/data

  kafka:
    image: confluentinc/cp-kafka:7.5.0
    ports:
      - "9092:9092"
    environment:
      KAFKA_BROKER_ID: 1
      KAFKA_ZOOKEEPER_CONNECT: zookeeper:2181
      KAFKA_ADVERTISED_LISTENERS: PLAINTEXT://kafka:9092,PLAINTEXT_HOST://localhost:29092
      KAFKA_LISTENER_SECURITY_PROTOCOL_MAP: PLAINTEXT:PLAINTEXT,PLAINTEXT_HOST:PLAINTEXT
      KAFKA_INTER_BROKER_LISTENER_NAME: PLAINTEXT
      KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR: 1
      KAFKA_AUTO_CREATE_TOPICS_ENABLE: "true"
    depends_on:
      - zookeeper
    volumes:
      - kafka-data:/var/lib/kafka/data
    healthcheck:
      test: ["CMD", "kafka-broker-api-versions", "--bootstrap-server", "localhost:9092"]
      interval: 10s
      timeout: 10s
      retries: 5

  # Kafka UI (Development)
  kafka-ui:
    image: provectuslabs/kafka-ui:latest
    ports:
      - "8080:8080"
    environment:
      KAFKA_CLUSTERS_0_NAME: local
      KAFKA_CLUSTERS_0_BOOTSTRAPSERVERS: kafka:9092
    depends_on:
      - kafka

  # OpenTelemetry Collector
  otel-collector:
    image: otel/opentelemetry-collector-contrib:0.91.0
    ports:
      - "4317:4317"   # OTLP gRPC
      - "4318:4318"   # OTLP HTTP
      - "8888:8888"   # Prometheus metrics (collector)
      - "8889:8889"   # Prometheus exporter
    volumes:
      - ./otel-collector-config.yaml:/etc/otelcol-contrib/config.yaml
    command: ["--config=/etc/otelcol-contrib/config.yaml"]
    depends_on:
      - jaeger
      - prometheus

  # Jaeger (Distributed Tracing)
  jaeger:
    image: jaegertracing/all-in-one:1.52
    ports:
      - "16686:16686"  # UI
      - "14268:14268"  # HTTP collector
      - "14250:14250"  # gRPC collector
    environment:
      COLLECTOR_OTLP_ENABLED: "true"

  # Prometheus (Metrics)
  prometheus:
    image: prom/prometheus:v2.48.0
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.enable-lifecycle'

  # Grafana (Dashboards)
  grafana:
    image: grafana/grafana:10.2.0
    ports:
      - "3000:3000"
    environment:
      GF_SECURITY_ADMIN_PASSWORD: admin
      GF_USERS_ALLOW_SIGN_UP: "false"
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/provisioning:/etc/grafana/provisioning
    depends_on:
      - prometheus
      - jaeger

  # MinIO (S3-compatible storage for development)
  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    command: server /data --console-address ":9001"
    volumes:
      - minio-data:/data

volumes:
  postgres-data:
  redis-data:
  kafka-data:
  zookeeper-data:
  prometheus-data:
  grafana-data:
  minio-data:
  cargo-cache:
```

### OpenTelemetry Collector Configuration

```yaml
# otel-collector-config.yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    timeout: 1s
    send_batch_size: 1024

  memory_limiter:
    check_interval: 1s
    limit_mib: 1000
    spike_limit_mib: 200

exporters:
  # Traces to Jaeger
  otlp/jaeger:
    endpoint: jaeger:4317
    tls:
      insecure: true

  # Metrics to Prometheus
  prometheus:
    endpoint: "0.0.0.0:8889"
    namespace: downloader

  # Logs to stdout (or configure for Loki/Elasticsearch)
  logging:
    loglevel: info

  # Optional: Export to Kafka for event replay
  kafka:
    brokers:
      - kafka:9092
    topic: otel-traces
    encoding: otlp_json

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [memory_limiter, batch]
      exporters: [otlp/jaeger, kafka]

    metrics:
      receivers: [otlp]
      processors: [memory_limiter, batch]
      exporters: [prometheus]

    logs:
      receivers: [otlp]
      processors: [memory_limiter, batch]
      exporters: [logging]
```

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  # Scrape OpenTelemetry Collector's Prometheus exporter
  - job_name: 'otel-collector'
    static_configs:
      - targets: ['otel-collector:8889']

  # Scrape application directly if exposing /metrics
  - job_name: 'downloader-rs'
    static_configs:
      - targets: ['downloader-rs:8000']
    metrics_path: '/metrics'
```

---

*This implementation plan is a living document. Update it as requirements evolve and lessons are learned during development.*
