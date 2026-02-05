#!/usr/bin/env bash
# Development environment setup script for downloader-rs
# Usage: ./scripts/setup.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_command() {
    if ! command -v "$1" &> /dev/null; then
        return 1
    fi
    return 0
}

# Header
echo ""
echo "=================================="
echo "  downloader-rs Development Setup"
echo "=================================="
echo ""

# Check Rust installation
log_info "Checking Rust installation..."
if check_command rustc; then
    RUST_VERSION=$(rustc --version)
    log_success "Rust is installed: $RUST_VERSION"
else
    log_error "Rust is not installed. Please install from https://rustup.rs/"
    exit 1
fi

# Check cargo
if check_command cargo; then
    CARGO_VERSION=$(cargo --version)
    log_success "Cargo is installed: $CARGO_VERSION"
else
    log_error "Cargo is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Install Rust components
log_info "Installing Rust components..."
rustup component add rustfmt clippy llvm-tools-preview 2>/dev/null || true
log_success "Rust components installed"

# Check Docker
log_info "Checking Docker installation..."
if check_command docker; then
    DOCKER_VERSION=$(docker --version)
    log_success "Docker is installed: $DOCKER_VERSION"
else
    log_warn "Docker is not installed. Some features will not work."
    log_warn "Install from https://docs.docker.com/get-docker/"
fi

# Check Docker Compose
if check_command docker && docker compose version &> /dev/null; then
    COMPOSE_VERSION=$(docker compose version --short)
    log_success "Docker Compose is installed: $COMPOSE_VERSION"
else
    log_warn "Docker Compose is not available."
fi

# Install cargo tools
log_info "Installing cargo development tools..."

# sqlx-cli
if ! check_command sqlx; then
    log_info "Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features rustls,postgres
    log_success "sqlx-cli installed"
else
    log_success "sqlx-cli is already installed"
fi

# cargo-llvm-cov for coverage
if ! cargo llvm-cov --version &> /dev/null 2>&1; then
    log_info "Installing cargo-llvm-cov..."
    cargo install cargo-llvm-cov
    log_success "cargo-llvm-cov installed"
else
    log_success "cargo-llvm-cov is already installed"
fi

# cargo-deny for security
if ! check_command cargo-deny; then
    log_info "Installing cargo-deny..."
    cargo install cargo-deny
    log_success "cargo-deny installed"
else
    log_success "cargo-deny is already installed"
fi

# cargo-watch for auto-reload (optional)
if ! check_command cargo-watch; then
    log_info "Installing cargo-watch..."
    cargo install cargo-watch
    log_success "cargo-watch installed"
else
    log_success "cargo-watch is already installed"
fi

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    log_info "Creating .env file..."
    cat > .env << 'EOF'
# Development environment configuration
DATABASE_URL=postgres://postgres:password@localhost:5432/downloader
REDIS_URL=redis://localhost:6379
APP_ENVIRONMENT=local
RUST_LOG=debug
TEST_LOG=false
EOF
    log_success ".env file created"
else
    log_success ".env file already exists"
fi

# Start Docker services if available
if check_command docker && docker compose version &> /dev/null; then
    log_info "Starting Docker services..."
    docker compose -f docker-compose.local.yml up -d postgres redis 2>/dev/null || {
        log_warn "Could not start Docker services. Make sure Docker is running."
    }

    # Wait for PostgreSQL to be ready
    log_info "Waiting for PostgreSQL to be ready..."
    for i in {1..30}; do
        if docker compose -f docker-compose.local.yml exec -T postgres pg_isready -U postgres &> /dev/null; then
            log_success "PostgreSQL is ready"
            break
        fi
        if [ $i -eq 30 ]; then
            log_warn "PostgreSQL did not become ready in time"
        fi
        sleep 1
    done
fi

# Run database migrations
if check_command sqlx; then
    log_info "Running database migrations..."
    if sqlx migrate run 2>/dev/null; then
        log_success "Database migrations completed"
    else
        log_warn "Could not run migrations. Database may not be ready."
    fi
fi

# Prepare SQLx offline data
log_info "Preparing SQLx offline data..."
if cargo sqlx prepare --workspace 2>/dev/null; then
    log_success "SQLx offline data prepared"
else
    log_warn "Could not prepare SQLx data. Database may not be running."
fi

# Build the project
log_info "Building the project..."
if cargo build; then
    log_success "Project built successfully"
else
    log_error "Build failed"
    exit 1
fi

# Run tests
log_info "Running tests..."
if cargo test --lib 2>/dev/null; then
    log_success "Unit tests passed"
else
    log_warn "Some tests failed"
fi

# Summary
echo ""
echo "=================================="
echo "  Setup Complete!"
echo "=================================="
echo ""
echo "Next steps:"
echo "  1. Start the development server:  make run"
echo "  2. Run all tests:                 make test"
echo "  3. Check code quality:            make check"
echo "  4. View available commands:       make help"
echo ""
echo "Useful commands:"
echo "  make dev          - Start dev environment (db, redis)"
echo "  make watch        - Run with auto-reload"
echo "  make coverage     - Generate test coverage report"
echo "  make docs-open    - Generate and open documentation"
echo ""
