# Claude Code Guide for downloader-rs

## Project Overview

**downloader-rs** is a Rust-based web application providing file downloading capabilities with user authentication, database management, and optional S3 storage integration. It operates in two modes:

1. **CLI Mode**: Direct file downloads from command line
2. **API Mode**: REST API server with authentication and download management

### Tech Stack
- **Runtime**: Tokio async runtime
- **Web Framework**: Actix-web 4.x
- **Database**: PostgreSQL with SQLx (compile-time verified queries)
- **Session Store**: Redis with actix-session
- **Authentication**: Dual JWT + Session-based auth
- **Password Hashing**: Argon2id
- **Logging**: tracing + tracing-bunyan-formatter (structured JSON)
- **File Downloads**: manic crate (multi-threaded)
- **Cloud Storage**: rust-s3 (optional)

---

## Project Structure

```
downloader-rs/
├── src/
│   ├── main.rs              # Entry point - CLI args parsing, mode selection
│   ├── lib.rs               # Library root exports
│   ├── api.rs               # Actix server setup, middleware stack, route registration
│   ├── configuration.rs     # YAML + env config loading
│   ├── session_state.rs     # TypedSession wrapper for Redis sessions
│   ├── telemetry.rs         # Tracing/logging setup
│   ├── cli/
│   │   └── mod.rs           # Clap CLI argument definitions
│   ├── models/
│   │   ├── mod.rs
│   │   ├── user.rs          # User struct with SecretString password
│   │   ├── download.rs      # Download struct with status enum
│   │   ├── pagination.rs    # PaginationParams, PaginationMeta, PaginatedResponse
│   │   ├── filter.rs        # DownloadFilter for query filtering
│   │   ├── sorting.rs       # SortParams, SortOrder, field whitelists
│   │   └── query.rs         # DownloadQueryParams (combined query params)
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── auth.rs          # POST /auth (login), POST /register
│   │   ├── download.rs      # Download endpoints (protected)
│   │   └── health_check.rs  # GET /health_check
│   ├── repository/
│   │   ├── mod.rs
│   │   ├── auth.rs          # User CRUD operations
│   │   └── download.rs      # Download CRUD with pagination support
│   ├── middlewares/
│   │   ├── mod.rs
│   │   └── auth.rs          # reject_anonymous_users middleware
│   ├── clients/
│   │   ├── mod.rs
│   │   └── s3.rs            # S3 bucket operations
│   └── utils/
│       ├── mod.rs
│       ├── api.rs           # Error response helpers (e400, e401, e404, e500)
│       ├── auth.rs          # Password hashing/verification
│       ├── errors.rs        # Custom error types (AuthError, LoginError)
│       └── file.rs          # File download utilities
├── migrations/              # SQLx migrations (up/down SQL files)
├── configuration/
│   ├── base.yaml            # Shared config
│   ├── local.yaml           # Development overrides
│   └── production.yaml      # Production overrides
├── tests/
│   └── api/
│       ├── main.rs          # Test entry point
│       ├── helpers.rs       # TestApp, spawn_app(), test utilities
│       ├── fixtures.rs      # Test factories (DownloadFactory, UserFactory)
│       ├── health_check.rs
│       ├── auth.rs
│       └── download.rs
├── scripts/
│   ├── init_db.sh           # Database initialization
│   └── setup.sh             # One-command dev environment setup
├── compose/                 # Docker compose files
├── Makefile                 # Development commands (30+ targets)
├── rust-toolchain.toml      # Rust version pinning
├── rustfmt.toml             # Code formatting rules
├── deny.toml                # Dependency license/security checking
├── ARCHITECTURE.md          # System architecture documentation
└── CONTRIBUTING.md          # Contribution guidelines
```

---

## Key Architecture Patterns

### Layered Architecture
```
Routes (HTTP handlers) → Repository (data access) → Database
                      ↘ Utils (business logic)
                      ↘ Clients (external services)
```

### Request Flow
```
Request → NormalizePath → SessionMiddleware → TracingLogger → Router
       → Auth Middleware (for protected routes) → Handler → Response
```

### Authentication Strategy
The middleware (`src/middlewares/auth.rs`) implements dual authentication:
1. **JWT** (primary): Checks `Authorization: Bearer <token>` header
2. **Session** (fallback): Checks Redis-backed session cookie

```rust
// Pattern for protected route handlers - user_id from middleware
pub async fn handler(
    user_id: web::ReqData<UserId>,  // Injected by auth middleware
    pool: web::Data<PgPool>,
    // ...
) -> Result<HttpResponse, actix_web::Error>
```

### Error Handling Convention
Use the error helper functions from `src/utils/api.rs`:
```rust
use crate::utils::api::{e400, e401, e404, e500};

// In handlers:
repository::get_user(&pool, id).await.map_err(e500)?;
```

All errors return JSON: `{"error": "message"}`

---

## Database

### Running Migrations
```bash
# Install sqlx-cli if not present
cargo install sqlx-cli --no-default-features --features rustls,postgres

# Run migrations
sqlx migrate run

# Create new migration
sqlx migrate add <name>
```

### Models Location
- `src/models/user.rs` - User with email, password_hash, timestamps
- `src/models/download.rs` - Download with url, status, file_path, user_id
- `src/models/pagination.rs` - Pagination structs for list endpoints
- `src/models/filter.rs` - Filtering structs for query building
- `src/models/sorting.rs` - Sorting structs with field whitelisting
- `src/models/query.rs` - Combined query parameters

### Key Model Types
```rust
// Pagination
pub struct PaginationParams { page: u32, per_page: u32, cursor: Option<String> }
pub struct PaginatedResponse<T> { data: Vec<T>, pagination: PaginationMeta }

// Filtering
pub struct DownloadFilter { status, url_contains, created_after, created_before, ... }

// Sorting
pub struct SortParams { sort_by: Option<String>, sort_order: SortOrder }
pub enum SortOrder { Asc, Desc }

// Combined
pub struct DownloadQueryParams { /* all pagination, filter, sort fields */ }
```

### Repository Pattern
All database operations go through `src/repository/`:
- Never write raw SQL in route handlers
- Use sqlx macros (`sqlx::query!`, `sqlx::query_as!`) for compile-time verification
- Return `anyhow::Result` from repository functions

---

## Configuration

### Loading Order
1. `configuration/base.yaml` (always loaded)
2. `configuration/{environment}.yaml` (local or production)
3. Environment variables with `APP_` prefix

### Environment Detection
Set `APP_ENVIRONMENT=local` or `APP_ENVIRONMENT=production`

### Key Configuration Sections
```yaml
application:
  port: 8000
  host: "0.0.0.0"
  hmac_secret: "..."
  jwt:
    secret: "..."
    expiration_hours: 7

database:
  host: "localhost"
  port: 5432
  username: "postgres"
  password: "password"
  database_name: "downloader"
  require_ssl: false

redis_uri: "redis://localhost:6379"

s3:  # Optional
  bucket: "downloads"
  region: "us-east-1"
  endpoint: "http://localhost:9000"
  access_key: "..."
  secret_key: "..."
```

---

## Development Commands

### Running Locally
```bash
# Start dependencies (Postgres, Redis)
docker compose -f docker-compose.local.yml up -d

# Run migrations
sqlx migrate run

# Run in API mode
cargo run -- --api

# Run in CLI mode (download files)
cargo run -- -l "https://example.com/file.zip"
```

### Testing
```bash
# Run all tests
cargo test

# Run with logging output
TEST_LOG=true cargo test

# Run specific test
cargo test test_name
```

### Linting & Formatting
```bash
cargo fmt
cargo clippy
```

### Database Preparation (for CI)
```bash
# Prepare sqlx offline data
cargo sqlx prepare
```

---

## API Endpoints

### Public Endpoints
| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/auth` | Login with email/password, returns JWT |
| POST | `/api/v1/register` | Register new user, returns JWT |
| GET | `/api/v1/health_check` | Health check (200 OK) |

### Protected Endpoints (require auth)
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/download_file?url=` | Initiate file download |
| GET | `/api/v1/downloads/{id}` | Get single download by ID |
| GET | `/api/v1/downloads` | List user's downloads (paginated) |

### Authentication Header
```
Authorization: Bearer <jwt_token>
```

### Pagination, Filtering & Sorting

The `GET /api/v1/downloads` endpoint supports the following query parameters:

**Pagination:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `page` | u32 | 1 | Page number (1-indexed) |
| `per_page` | u32 | 20 | Items per page (max 100) |
| `cursor` | string | - | Cursor for cursor-based pagination |

**Filtering:**
| Parameter | Type | Description |
|-----------|------|-------------|
| `status` | string | Filter by status: PENDING, IN_PROGRESS, COMPLETED, FAILED |
| `url_contains` | string | Filter by URL containing string (case-insensitive) |
| `created_after` | ISO 8601 | Filter downloads created after date |
| `created_before` | ISO 8601 | Filter downloads created before date |

**Sorting:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `sort_by` | string | created_at | Field: created_at, updated_at, completed_at, status, url |
| `sort_order` | string | desc | Order: asc, desc |

**Example Request:**
```
GET /api/v1/downloads?page=1&per_page=10&status=COMPLETED&sort_by=created_at&sort_order=desc
```

**Response Format:**
```json
{
  "data": [...],
  "pagination": {
    "page": 1,
    "per_page": 10,
    "total": 45,
    "total_pages": 5,
    "has_next": true,
    "has_prev": false
  }
}
```

---

## Adding New Features

### Adding a New Route
1. Create handler function in `src/routes/` (new file or existing)
2. Export from `src/routes/mod.rs`
3. Register route in `src/api.rs` within the appropriate scope
4. Add tests in `tests/api/`

### Adding a New Model
1. Create struct in `src/models/` with sqlx::FromRow derive
2. Create migration in `migrations/`
3. Create repository functions in `src/repository/`
4. Export from respective `mod.rs` files

### Adding Protected Route
Register within the protected scope that has `reject_anonymous_users` middleware:
```rust
// In src/api.rs
.service(
    web::scope("")
        .wrap(from_fn(reject_anonymous_users))
        .route("/your_route", web::get().to(your_handler))
)
```

---

## Testing Conventions

### Test Structure
Tests use an isolated database per test via `spawn_app()`:
```rust
#[tokio::test]
async fn test_something() {
    let app = spawn_app().await;
    // app.address - server URL
    // app.api_client - reqwest client with cookies
    // app.db_pool - direct DB access
    // app.test_user - pre-created authenticated user
}
```

### Making Authenticated Requests in Tests
```rust
let response = app.api_client
    .get(&format!("{}/api/v1/downloads", &app.address))
    .header("Authorization", format!("Bearer {}", app.test_user.jwt))
    .send()
    .await
    .expect("Failed to execute request.");
```

---

## Security Considerations

- **Password hashing**: Argon2id with memory-hard parameters (see `src/utils/auth.rs`)
- **Secrets**: Use `SecretString` from `secrecy` crate for sensitive data
- **SQL injection**: Prevented by sqlx prepared statements
- **Timing attacks**: Constant-time password comparison
- **JWT**: Short expiration (7 hours default), validated on each request

---

## Common Patterns

### Extracting User ID in Handlers
```rust
pub async fn handler(user_id: web::ReqData<UserId>) -> impl Responder {
    let user_id = user_id.into_inner();
    // use user_id.0 (the Uuid)
}
```

### Database Queries with Error Mapping
```rust
let download = repository::get_download(&pool, id)
    .await
    .map_err(e500)?
    .ok_or_else(|| e404(anyhow::anyhow!("Download not found")))?;
```

### Returning JSON Responses
```rust
Ok(HttpResponse::Ok().json(data))
```

---

## Implementation Roadmap

A comprehensive **16-week implementation plan** is available in [IMPLEMENTATION_PLAN.md](./IMPLEMENTATION_PLAN.md).

### Implementation Status

| Phase | Weeks | Focus Areas | Status |
|-------|-------|-------------|--------|
| 1 | 1-2 | CI/CD, Testing Infrastructure, Documentation | ✅ Complete |
| 2 | 3-4 | API Enhancements (Pagination, Filtering, Sorting) | ✅ Complete |
| 3 | 5-6 | User Management (Profile, Password Reset, Email Verification) | 🔲 Pending |
| 4 | 7-8 | Download Management (Progress, Pause/Resume, Retry) | 🔲 Pending |
| 5 | 9-10 | Background Jobs (Redis Streams Queue System) | 🔲 Pending |
| 6 | 11-12 | File Organization (Folders, Tags, Search) | 🔲 Pending |
| 7 | 13-14 | Storage Options & Rate Limiting | 🔲 Pending |
| 8 | 15-16 | Monitoring (OpenTelemetry, Kafka) & Admin Features | 🔲 Pending |

### Phase 1 Deliverables (Complete)
- ✅ `rust-toolchain.toml` - Rust version pinning (stable)
- ✅ `rustfmt.toml` - Code formatting rules
- ✅ `deny.toml` - Dependency license/security checking
- ✅ Clippy lints configured in `Cargo.toml`
- ✅ `Makefile` with 30+ development targets
- ✅ `scripts/setup.sh` - One-command dev environment setup
- ✅ `CONTRIBUTING.md` - Contribution guidelines
- ✅ `ARCHITECTURE.md` - System architecture documentation
- ✅ `tests/api/fixtures.rs` - Test factories (DownloadFactory, UserFactory)
- 🔲 GitHub Actions CI workflows (can be added when needed)
- 🔲 OpenAPI/Swagger spec (can be added when needed)

### Phase 2 Deliverables (Complete)
- ✅ `src/models/pagination.rs` - PaginationParams, PaginationMeta, PaginatedResponse
- ✅ `src/models/filter.rs` - DownloadFilter with status, URL, date range filtering
- ✅ `src/models/sorting.rs` - SortParams, SortOrder, field whitelisting
- ✅ `src/models/query.rs` - DownloadQueryParams (unified query parameters)
- ✅ Updated `src/repository/download.rs` with `get_downloads_paginated()`
- ✅ Updated `GET /downloads` endpoint to return paginated responses
- ✅ Added Clone, Copy, PartialEq, Eq derives to DownloadStatus
- ✅ Unit tests for pagination, filtering, sorting

### Feature Areas Covered
1. **Download Management**: Progress tracking, pause/resume, retry logic
2. **User Management**: Profile updates, password reset, email verification
3. **File Organization**: Folders, tags, search functionality
4. **Notifications**: Webhooks, email notifications on completion
5. **Rate Limiting**: Per-user download limits, bandwidth throttling
6. **Admin Features**: User management, system metrics, audit logs
7. **API Enhancements**: Pagination, filtering, sorting for list endpoints
8. **Storage Options**: Additional cloud providers, local storage management
9. **Background Jobs**: Redis Streams queue system for async download processing
10. **Monitoring**: OpenTelemetry + Kafka for metrics, traces, and events

See the full plan for detailed daily breakdowns, code examples, database migrations, and test requirements.

---

## Troubleshooting

### SQLx Compile Errors
If seeing "database not found" during compilation:
```bash
# Ensure DATABASE_URL is set or .env file exists
export DATABASE_URL="postgres://postgres:password@localhost:5432/downloader"

# Or use offline mode
cargo sqlx prepare
```

### Test Database Issues
Tests create temporary databases. If tests fail with DB errors:
```bash
# Ensure Postgres is running
docker compose -f docker-compose.local.yml up -d postgres

# Check for orphaned test databases and clean up
psql -U postgres -c "SELECT datname FROM pg_database WHERE datname LIKE 'test_%';"
```
