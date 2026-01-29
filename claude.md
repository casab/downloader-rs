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
│   │   └── download.rs      # Download struct with status enum
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── auth.rs          # POST /auth (login), POST /register
│   │   ├── download.rs      # Download endpoints (protected)
│   │   └── health_check.rs  # GET /health_check
│   ├── repository/
│   │   ├── mod.rs
│   │   ├── auth.rs          # User CRUD operations
│   │   └── download.rs      # Download CRUD operations
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
│       ├── health_check.rs
│       ├── auth.rs
│       └── download.rs
└── compose/                 # Docker compose files
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
| GET | `/api/v1/downloads` | List user's downloads |

### Authentication Header
```
Authorization: Bearer <jwt_token>
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

## Potential Feature Areas

When planning new features, consider these areas:

1. **Download Management**: Progress tracking, pause/resume, retry logic
2. **User Management**: Profile updates, password reset, email verification
3. **File Organization**: Folders, tags, search functionality
4. **Notifications**: Webhooks, email notifications on completion
5. **Rate Limiting**: Per-user download limits, bandwidth throttling
6. **Admin Features**: User management, system metrics, audit logs
7. **API Enhancements**: Pagination, filtering, sorting for list endpoints
8. **Storage Options**: Additional cloud providers, local storage management
9. **Background Jobs**: Queue system for async download processing
10. **Monitoring**: Prometheus metrics, health check enhancements

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
