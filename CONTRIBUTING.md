# Contributing to downloader-rs

Thank you for your interest in contributing to downloader-rs! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Style](#code-style)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Commit Messages](#commit-messages)

## Code of Conduct

This project adheres to a code of conduct. By participating, you are expected to uphold this code. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust (stable, see `rust-toolchain.toml` for exact version)
- Docker and Docker Compose
- PostgreSQL client (for development)

### Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/casab/downloader-rs.git
   cd downloader-rs
   ```

2. Run the setup script:
   ```bash
   ./scripts/setup.sh
   ```

   This will:
   - Install required Rust components
   - Install development tools (sqlx-cli, cargo-llvm-cov, cargo-deny)
   - Create a `.env` file
   - Start Docker services
   - Run database migrations
   - Build the project

3. Verify the setup:
   ```bash
   make check
   ```

## Development Workflow

### Running the Application

```bash
# Start development environment (PostgreSQL, Redis)
make dev

# Run the API server
make run

# Run with auto-reload (recommended for development)
make watch
```

### Common Commands

| Command | Description |
|---------|-------------|
| `make build` | Build the project |
| `make test` | Run all tests |
| `make lint` | Run Clippy lints |
| `make fmt` | Format code |
| `make check` | Run all quality checks |
| `make coverage` | Generate coverage report |
| `make docs` | Generate documentation |
| `make help` | Show all available commands |

### Database Operations

```bash
# Run migrations
make db-migrate

# Create a new migration
make db-migration

# Reset database
make db-reset

# Prepare SQLx offline data (required before committing)
make prepare
```

## Code Style

### Formatting

We use `rustfmt` for code formatting. Configuration is in `rustfmt.toml`.

```bash
# Check formatting
make fmt-check

# Apply formatting
make fmt
```

### Linting

We use Clippy with strict settings. Configuration is in `Cargo.toml` under `[lints.clippy]`.

```bash
# Run lints
make lint
```

### Key Style Guidelines

1. **Error Handling**: Use `anyhow::Result` for application errors, custom error types for library code.

2. **Logging**: Use the `tracing` crate for structured logging.
   ```rust
   tracing::info!(user_id = %id, "User logged in");
   ```

3. **Documentation**: Add doc comments for all public items.
   ```rust
   /// Creates a new download for the specified URL.
   ///
   /// # Arguments
   /// * `url` - The URL to download
   /// * `user_id` - The ID of the user initiating the download
   ///
   /// # Returns
   /// The created download record
   pub async fn create_download(url: &str, user_id: Uuid) -> Result<Download> {
       // ...
   }
   ```

4. **Async Code**: Prefer `async`/`await` over callbacks.

5. **Secrets**: Use `SecretString` for sensitive data.

## Testing

### Running Tests

```bash
# All tests
make test

# Unit tests only (fast, no database)
make test-unit

# Integration tests only
make test-integration

# With coverage report
make coverage
```

### Writing Tests

1. **Unit Tests**: Place in the same file as the code being tested.
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_something() {
           // ...
       }
   }
   ```

2. **Integration Tests**: Place in `tests/api/`.
   ```rust
   #[tokio::test]
   async fn test_endpoint() {
       let app = spawn_app().await;
       // ...
   }
   ```

3. **Test Coverage**: Aim for >80% coverage on new code.

### Test Helpers

Use the test helpers in `tests/api/helpers.rs`:

```rust
// Spawn a test application
let app = spawn_app().await;

// Make authenticated requests
let response = app.api_client
    .get(&format!("{}/api/v1/downloads", &app.address))
    .header("Authorization", format!("Bearer {}", app.test_user.jwt))
    .send()
    .await
    .expect("Failed to execute request.");
```

## Pull Request Process

### Before Submitting

1. **Run all checks locally**:
   ```bash
   make ci
   ```

2. **Update documentation** if you've changed public APIs.

3. **Add tests** for new functionality.

4. **Update SQLx offline data** if you've changed database queries:
   ```bash
   make prepare
   ```

### PR Guidelines

1. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Keep PRs focused**: One feature or fix per PR.

3. **Write a clear description**: Explain what changes you made and why.

4. **Link related issues**: Use "Fixes #123" or "Relates to #123".

5. **Request review**: Tag relevant reviewers.

### Review Process

- All PRs require at least one approval
- CI must pass before merging
- Squash and merge is preferred for clean history

## Commit Messages

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification.

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, no code change
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvement
- `test`: Adding or updating tests
- `chore`: Build process or auxiliary tool changes

### Examples

```
feat(download): add pause/resume functionality

Implement HTTP Range request support for resumable downloads.
Add new endpoints POST /downloads/{id}/pause and /resume.

Closes #42
```

```
fix(auth): handle expired JWT tokens gracefully

Return 401 with clear error message instead of 500 when
JWT token has expired.
```

## Questions?

If you have questions, feel free to:

1. Open a [GitHub Discussion](https://github.com/casab/downloader-rs/discussions)
2. Check existing issues for similar questions
3. Review the [Architecture Guide](./ARCHITECTURE.md)

Thank you for contributing!
