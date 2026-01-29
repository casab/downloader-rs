# Makefile for downloader-rs
# Run `make help` to see available targets

.PHONY: all build test lint fmt check clean coverage docs help
.PHONY: dev db-start db-stop db-migrate db-reset docker-build docker-run
.PHONY: ci audit prepare

# Default target
all: fmt lint test

# ============================================================================
# Development
# ============================================================================

## Build the project in debug mode
build:
	cargo build

## Build the project in release mode
build-release:
	cargo build --release

## Run the application in API mode
run:
	cargo run -- --api

## Run the application with auto-reload (requires cargo-watch)
watch:
	cargo watch -x 'run -- --api'

## Start development environment (database, redis)
dev: db-start
	@echo "Development environment started"
	@echo "Run 'make run' to start the application"

# ============================================================================
# Testing
# ============================================================================

## Run all tests
test:
	cargo test --all-features

## Run tests with output
test-verbose:
	cargo test --all-features -- --nocapture

## Run only unit tests (no database required)
test-unit:
	cargo test --lib

## Run only integration tests
test-integration:
	cargo test --test '*'

## Run tests with coverage report
coverage:
	cargo llvm-cov --all-features --workspace --html --output-dir coverage
	@echo "Coverage report generated in coverage/html/index.html"

## Run tests with coverage and open report
coverage-open: coverage
	@if command -v xdg-open > /dev/null; then \
		xdg-open coverage/html/index.html; \
	elif command -v open > /dev/null; then \
		open coverage/html/index.html; \
	fi

# ============================================================================
# Code Quality
# ============================================================================

## Run all lints (clippy)
lint:
	cargo clippy --all-targets --all-features -- -D warnings

## Check code formatting
fmt-check:
	cargo fmt --all -- --check

## Format code
fmt:
	cargo fmt --all

## Run all checks (format, lint, test)
check: fmt-check lint test
	@echo "All checks passed!"

## Run cargo check (fast compile check)
check-compile:
	cargo check --all-targets --all-features

## Run security audit
audit:
	cargo deny check

## Run full audit (advisories, licenses, bans, sources)
audit-full:
	cargo deny check advisories licenses bans sources

# ============================================================================
# Database
# ============================================================================

## Start database containers
db-start:
	docker compose -f docker-compose.local.yml up -d postgres redis

## Stop database containers
db-stop:
	docker compose -f docker-compose.local.yml down

## Run database migrations
db-migrate:
	sqlx migrate run

## Create a new migration
db-migration:
	@read -p "Migration name: " name; \
	sqlx migrate add $$name

## Reset database (drop and recreate)
db-reset:
	sqlx database drop -y || true
	sqlx database create
	sqlx migrate run

## Prepare SQLx offline data
prepare:
	cargo sqlx prepare --workspace

## Check SQLx queries are up to date
prepare-check:
	cargo sqlx prepare --workspace --check

# ============================================================================
# Docker
# ============================================================================

## Build Docker image
docker-build:
	docker build -t downloader-rs:latest .

## Run application in Docker
docker-run:
	docker compose -f docker-compose.local.yml up

## Run application in Docker (detached)
docker-run-d:
	docker compose -f docker-compose.local.yml up -d

## Stop Docker containers
docker-stop:
	docker compose -f docker-compose.local.yml down

## View Docker logs
docker-logs:
	docker compose -f docker-compose.local.yml logs -f

# ============================================================================
# Documentation
# ============================================================================

## Generate documentation
docs:
	cargo doc --no-deps --document-private-items

## Generate and open documentation
docs-open:
	cargo doc --no-deps --document-private-items --open

# ============================================================================
# CI Helpers
# ============================================================================

## Run CI checks locally (mirrors GitHub Actions)
ci: fmt-check lint test prepare-check
	@echo "CI checks passed!"

## Install development dependencies
install-deps:
	cargo install sqlx-cli --no-default-features --features rustls,postgres
	cargo install cargo-llvm-cov
	cargo install cargo-deny
	cargo install cargo-watch

# ============================================================================
# Cleanup
# ============================================================================

## Clean build artifacts
clean:
	cargo clean
	rm -rf coverage/

## Clean everything including database
clean-all: clean db-stop
	docker volume rm downloader-rs_postgres-data 2>/dev/null || true
	docker volume rm downloader-rs_redis-data 2>/dev/null || true

# ============================================================================
# Help
# ============================================================================

## Show this help message
help:
	@echo "Available targets:"
	@echo ""
	@awk '/^## /{desc=substr($$0,4)} /^[a-zA-Z][a-zA-Z0-9_-]+:/{gsub(/:.*/, "", $$1); printf "  \033[36m%-20s\033[0m %s\n", $$1, desc; desc=""}' $(MAKEFILE_LIST)
	@echo ""
	@echo "Examples:"
	@echo "  make dev          # Start development environment"
	@echo "  make run          # Run the application"
	@echo "  make test         # Run all tests"
	@echo "  make check        # Run all quality checks"
	@echo "  make ci           # Run CI checks locally"
