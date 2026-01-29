# downloader-rs: 16-Week Implementation Plan

## Executive Summary

This document outlines a comprehensive 16-week development roadmap for enhancing downloader-rs with 10 major feature areas. The plan is structured in phases that respect dependencies, maintain code quality through continuous testing and linting, and deliver incremental value.

### Feature Areas Covered
1. CI/CD & Development Infrastructure
2. API Enhancements (Pagination, Filtering, Sorting)
3. User Management (Profile, Password Reset, Email Verification)
4. Download Management (Progress, Pause/Resume, Retry)
5. Background Jobs (Queue System)
6. File Organization (Folders, Tags, Search)
7. Storage Options (Multi-provider Support)
8. Rate Limiting (Per-user Limits, Throttling)
9. Monitoring (Prometheus, Health Checks)
10. Admin Features (User Management, Audit Logs)
11. Notifications (Webhooks, Email)

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

## Phase 5: Background Jobs (Weeks 9-10)

### Week 9: Job Queue Infrastructure

#### Goals
- Implement job queue system
- Create worker pool architecture
- Add job persistence and recovery

#### Tasks

##### Day 1-2: Queue Infrastructure
| Task | Description | Tests Required |
|------|-------------|----------------|
| Evaluate queue backends | Redis vs PostgreSQL | Research |
| Create `jobs` table | Job persistence | Migration |
| Implement job serialization | Store job payloads | Unit tests |
| Create queue abstraction | `JobQueue` trait | Unit tests |

**Jobs table migration:**
```sql
-- migrations/YYYYMMDDHHMMSS_create_jobs_table.up.sql
CREATE TYPE job_status AS ENUM ('pending', 'running', 'completed', 'failed', 'cancelled');
CREATE TYPE job_type AS ENUM ('download', 'email', 'cleanup', 'notification');

CREATE TABLE jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_type job_type NOT NULL,
    status job_status NOT NULL DEFAULT 'pending',
    payload JSONB NOT NULL,
    result JSONB,
    priority INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    scheduled_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    failed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    worker_id TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_jobs_status_scheduled ON jobs(status, scheduled_at)
    WHERE status = 'pending';
CREATE INDEX idx_jobs_worker_id ON jobs(worker_id) WHERE status = 'running';
```

**Job queue trait:**
```rust
// src/jobs/queue.rs
#[async_trait]
pub trait JobQueue: Send + Sync {
    async fn enqueue(&self, job: Job) -> Result<Uuid>;
    async fn dequeue(&self, worker_id: &str) -> Result<Option<Job>>;
    async fn complete(&self, job_id: Uuid, result: JobResult) -> Result<()>;
    async fn fail(&self, job_id: Uuid, error: &str) -> Result<()>;
    async fn retry(&self, job_id: Uuid) -> Result<()>;
    async fn get_status(&self, job_id: Uuid) -> Result<JobStatus>;
}
```

##### Day 3-4: Worker Pool
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `Worker` struct | Job execution unit | Unit tests |
| Implement worker pool | Manage multiple workers | Integration tests |
| Add graceful shutdown | Complete running jobs | Integration tests |
| Worker heartbeat | Detect dead workers | Integration tests |

**Worker implementation:**
```rust
// src/jobs/worker.rs
pub struct Worker {
    id: String,
    queue: Arc<dyn JobQueue>,
    handlers: HashMap<JobType, Arc<dyn JobHandler>>,
    shutdown: Arc<AtomicBool>,
}

impl Worker {
    pub async fn run(&self) {
        while !self.shutdown.load(Ordering::SeqCst) {
            match self.queue.dequeue(&self.id).await {
                Ok(Some(job)) => self.process_job(job).await,
                Ok(None) => tokio::time::sleep(Duration::from_secs(1)).await,
                Err(e) => {
                    tracing::error!("Failed to dequeue job: {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}
```

##### Day 5: Job Types & Handlers
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `JobHandler` trait | Process specific job types | Unit tests |
| Implement download job handler | Execute downloads | Integration tests |
| Job result serialization | Store execution results | Unit tests |

#### Test Requirements - Week 9
```rust
#[tokio::test]
async fn job_enqueue_creates_pending_job() { }

#[tokio::test]
async fn worker_processes_pending_jobs() { }

#[tokio::test]
async fn failed_job_is_retried() { }

#[tokio::test]
async fn graceful_shutdown_completes_running_jobs() { }

#[tokio::test]
async fn dead_worker_jobs_are_recovered() { }
```

---

### Week 10: Download Queue Integration

#### Goals
- Integrate downloads with job queue
- Add job scheduling and prioritization
- Implement job monitoring

#### Tasks

##### Day 1-2: Download Job Integration
| Task | Description | Tests Required |
|------|-------------|----------------|
| Create `DownloadJobHandler` | Execute download jobs | Integration tests |
| Update `download_file` endpoint | Enqueue instead of execute | Integration tests |
| Progress updates from jobs | Publish progress events | Integration tests |
| Job-download relationship | Link job_id to download | Migration |

**Updated download flow:**
```rust
// src/routes/download.rs
pub async fn download(
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
    queue: web::Data<dyn JobQueue>,
    query: web::Query<DownloadQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    // Create download record
    let download = repository::create_download(&pool, &user_id, &query.url).await?;

    // Enqueue job
    let job = Job::new(JobType::Download, DownloadPayload {
        download_id: download.id,
        url: query.url.clone(),
        user_id: user_id.0,
    });
    queue.enqueue(job).await.map_err(e500)?;

    Ok(HttpResponse::Accepted().json(download))
}
```

##### Day 3-4: Job Scheduling & Priority
| Task | Description | Tests Required |
|------|-------------|----------------|
| Scheduled job execution | Future execution time | Integration tests |
| Priority queue | Higher priority first | Integration tests |
| Concurrent download limits | Per-user limits | Integration tests |
| Queue depth monitoring | Track pending jobs | Metrics |

**Priority levels:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Low = 0,
    Normal = 50,
    High = 100,
    Critical = 200,
}
```

##### Day 5: Job Monitoring Endpoints
| Task | Description | Tests Required |
|------|-------------|----------------|
| `GET /api/v1/jobs` | List user's jobs | Integration tests |
| `GET /api/v1/jobs/{id}` | Get job details | Integration tests |
| `POST /api/v1/jobs/{id}/cancel` | Cancel pending job | Integration tests |
| Job statistics | Queue depth, processing rate | Metrics |

#### Deliverables - Week 10
- [ ] Job queue system fully operational
- [ ] Downloads processed via job queue
- [ ] Priority-based scheduling
- [ ] Job monitoring endpoints
- [ ] Worker pool with graceful shutdown
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

## Phase 8: Monitoring & Admin (Weeks 15-16)

### Week 15: Monitoring & Observability

#### Goals
- Implement Prometheus metrics
- Enhance health checks
- Add system monitoring

#### Tasks

##### Day 1-2: Prometheus Metrics
| Task | Description | Tests Required |
|------|-------------|----------------|
| Add metrics endpoint | `GET /metrics` | Integration tests |
| HTTP request metrics | Duration, status codes | Automatic |
| Database metrics | Query duration, pool stats | Automatic |
| Business metrics | Downloads, users, jobs | Custom metrics |

**Metrics implementation:**
```rust
// src/metrics.rs
use prometheus::{
    Counter, Histogram, IntGauge, Registry,
    register_counter, register_histogram, register_int_gauge,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref HTTP_REQUESTS_TOTAL: Counter = register_counter!(
        "http_requests_total",
        "Total number of HTTP requests"
    ).unwrap();

    pub static ref HTTP_REQUEST_DURATION: Histogram = register_histogram!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds"
    ).unwrap();

    pub static ref DOWNLOADS_IN_PROGRESS: IntGauge = register_int_gauge!(
        "downloads_in_progress",
        "Number of downloads currently in progress"
    ).unwrap();

    pub static ref DOWNLOADS_TOTAL: Counter = register_counter!(
        "downloads_total",
        "Total number of downloads initiated"
    ).unwrap();

    pub static ref JOB_QUEUE_DEPTH: IntGauge = register_int_gauge!(
        "job_queue_depth",
        "Number of jobs waiting in queue"
    ).unwrap();
}
```

##### Day 3-4: Enhanced Health Checks
| Task | Description | Tests Required |
|------|-------------|----------------|
| Detailed health endpoint | `/api/v1/health` | Integration tests |
| Database health check | Connection test | Integration tests |
| Redis health check | Ping test | Integration tests |
| Storage health check | Access test | Integration tests |
| Dependency health aggregation | Overall status | Integration tests |

**Health check response:**
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
}

#[derive(Debug, Serialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}
```

##### Day 5: Alerting Foundation
| Task | Description | Tests Required |
|------|-------------|----------------|
| Alert rules definition | Prometheus alerting rules | Documentation |
| Grafana dashboard | Visualization templates | Manual setup |
| Log aggregation setup | Structured log shipping | Configuration |

#### Test Requirements - Week 15
```rust
#[tokio::test]
async fn metrics_endpoint_returns_prometheus_format() { }

#[tokio::test]
async fn health_check_reports_database_status() { }

#[tokio::test]
async fn health_check_reports_redis_status() { }

#[tokio::test]
async fn degraded_health_when_component_slow() { }
```

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

# Integration tests (require database)
cargo test --test '*'

# Specific feature tests
cargo test --features "integration" download_

# Performance/load tests
cargo test --release --features "benchmark"
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
| Background Jobs | Job completion rate > 99% |
| File Organization | Search latency < 200ms (p95) |
| Storage | Upload success rate > 99.9% |
| Monitoring | Alert accuracy > 95% |

### Final Success Criteria
- [ ] All 10 feature areas implemented
- [ ] Test coverage > 80%
- [ ] API documentation complete
- [ ] No critical security vulnerabilities
- [ ] Performance benchmarks met
- [ ] Admin features operational

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

*This implementation plan is a living document. Update it as requirements evolve and lessons are learned during development.*
