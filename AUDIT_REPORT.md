# Project Audit Report: downloader-rs

## 🔴 CRITICAL: Security Issues

### 1. Hardcoded Secrets in Configuration (HIGH RISK)
- **File:** `configuration/base.yaml:3-6`
- Hardcoded HMAC and JWT secrets are committed to version control
```yaml
hmac_secret: "long-and-very-secret-random-key-needed-to-verify-message-integrity"
jwt:
  secret: "another-long-and-very-secret-random-key-for-jwt"
```
- Production config (`production.yaml`) does **NOT** override these secrets

### 2. Sensitive Tokens Logged in Plaintext
- **File:** `src/routes/auth.rs:160-165, 284-289`
- Password reset and email verification tokens are logged with `tracing::info!`
```rust
tracing::info!(
    "Password reset token for {}: {} (expires: {})",
    email,
    token,  // <-- THE ACTUAL TOKEN IS LOGGED
    expires_at
);
```
- This is a severe vulnerability even with "REMOVE IN PRODUCTION" comments

### 3. No Rate Limiting on Auth Endpoints
- Login, registration, forgot-password endpoints have no rate limiting
- Vulnerable to brute force and credential stuffing attacks

---

## 🟠 HIGH: Rust Edition & Compilation Issues

### 4. Invalid Rust Edition
- **File:** `Cargo.toml:4`
```toml
edition = "2024"
```
- Rust 2024 edition is **not stable**. The latest stable edition is 2021. This may cause compilation issues.

---

## 🟠 HIGH: Potential Panics (Production Crashes)

### 5. `unwrap()` and `expect()` Usage

| File | Line | Issue |
|------|------|-------|
| `main.rs` | 15 | `expect("Failed to read configuration.")` |
| `api.rs` | 38 | `unwrap()` on `local_addr()` |
| `middlewares/auth.rs` | 60 | `expect("Failed to calculate expiration date")` |
| `middlewares/auth.rs` | 112 | `expect("JWT configuration must be set")` |
| `utils/auth.rs` | 30 | `Params::new(...).unwrap()` |

These will cause the server to crash rather than return an error.

---

## 🟡 MEDIUM: API Design Issues

### 6. Wrong HTTP Method for Download Creation
- **File:** `src/api.rs:118`
```rust
.route("/download_file", web::get().to(download))
```
- Uses GET for a write operation (creates a download record)
- Should be POST per REST conventions

### 7. Orphan Download Records on Failure
- **File:** `src/routes/download.rs:26-60`
- Creates download record BEFORE attempting download
- On failure, record remains with `Failed` status but user paid the cost

### 8. Download Control Doesn't Actually Work
- `pause_download`, `resume_download` only update database status
- The actual download process (using `manic`) is not interruptible
- **File:** `src/routes/download.rs:127-176`

---

## 🟡 MEDIUM: Database Issues

### 9. Missing Index on `users.email`
- **File:** `migrations/20250103063039_create_users_table.up.sql`
- Email lookups are common (login, registration check) but no index exists
```sql
CREATE TABLE IF NOT EXISTS users (
    email TEXT NOT NULL UNIQUE,  -- No explicit index
```

### 10. State Transition Validation Not Enforced
- **File:** `src/models/download.rs:141-159`
- `can_transition_to()` is defined but **never called** in repository operations
- Invalid state transitions are possible

---

## 🟡 MEDIUM: Code Quality Issues

### 11. Duplicate Password Update Functions
- `repository/auth.rs:60-82` - `change_password()`
- `repository/user.rs:120-142` - `update_password_hash()`
- Both do the same thing with slightly different implementations

### 12. Unused Code

| Item | Location | Issue |
|------|----------|-------|
| `LoginError` enum | `utils/errors.rs:9-15` | Defined but never used |
| `see_other()` function | `utils/api.rs:54-58` | Defined but never used |
| `get_all_downloads()` | `repository/download.rs:229-243` | Defined but never used |
| `verify_token()` | `utils/token.rs:35-37` | Exported but never used |

### 13. Silent Error Hiding
- **File:** `src/models/download.rs:200-212`
```rust
impl From<String> for DownloadStatus {
    fn from(status: String) -> Self {
        match status.to_uppercase().as_str() {
            ...
            _ => DownloadStatus::Failed,  // Hides invalid input!
        }
    }
}
```

---

## 🟡 MEDIUM: Missing Functionality

### 14. No Logout Endpoint
- Session-based auth has no way to explicitly log out via API
- `TypedSession::log_out()` exists but is never exposed

### 15. Progress Tracking Not Implemented
- `update_download_progress()` is defined but **never called**
- `bytes_downloaded` and `total_bytes` will always be 0/null

### 16. No Email Service Integration
- Password reset and email verification tokens have no delivery mechanism
- Currently just logged (and that's a security issue too)

---

## 🟢 LOW: Configuration Issues

### 17. Invalid Base URL
- **File:** `configuration/local.yaml:3`
```yaml
base_url: "http://0.0.0.0"
```
- `0.0.0.0` is not a valid URL for clients to use

### 18. No Environment Variable Validation
- Application starts even if critical environment variables are missing
- Secrets should be required via env vars in production

---

## 🟢 LOW: Test Issues

### 19. Test Database Cleanup Missing
- **File:** `tests/api/helpers.rs:147-167`
- Test databases are created but never dropped after tests
- Will accumulate orphan databases over time

### 20. Limited Test Coverage
- Only `download.rs` tests exist in `tests/api/`
- Missing tests for: auth, user profile, pagination, filtering, token operations

---

## 🟢 LOW: Inconsistencies

### 21. Inconsistent JSON Response Structure
- Some endpoints return `{"error": "..."}` (via `e400`, `e500`)
- Some endpoints return `{"message": "..."}` (success responses)
- Should use consistent structure

### 22. Unnecessary `return` Statements
- **File:** `src/routes/download.rs:41, 55-58`
```rust
return Ok(HttpResponse::Ok().finish());  // `return` is unnecessary
```

---

## Summary Table

| Severity | Count | Categories |
|----------|-------|------------|
| 🔴 Critical | 3 | Security |
| 🟠 High | 2 | Compilation, Panics |
| 🟡 Medium | 9 | API Design, Database, Code Quality, Missing Features |
| 🟢 Low | 6 | Configuration, Tests, Inconsistencies |
| **Total** | **20** | |

---

## Recommended Priority Fixes

1. Remove hardcoded secrets and require environment variables
2. Remove token logging in auth routes
3. Fix the Rust edition to 2021
4. Add rate limiting to authentication endpoints
