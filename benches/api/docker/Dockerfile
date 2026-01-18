# Dockerfile for Task Management Benchmark API
#
# Multi-stage build for optimized production image.
#
# Build stages:
#   1. builder: Compile the Rust application
#   2. runtime: Minimal runtime image with only the binary
#
# Usage:
#   docker compose build api
#   docker compose up -d

# =============================================================================
# Builder Stage
# =============================================================================

FROM rust:1.92.0-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the entire workspace (needed for lambars dependency)
COPY . .

# Build the API binary in release mode
RUN cargo build --release \
    --manifest-path benches/api/Cargo.toml \
    --bin task-management-benchmark-api

# =============================================================================
# Runtime Stage
# =============================================================================

FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -s /bin/bash appuser

# Copy the binary from builder
# Note: target directory is relative to the manifest location
COPY --from=builder /build/benches/api/target/release/task-management-benchmark-api /usr/local/bin/

# Set ownership
RUN chown appuser:appuser /usr/local/bin/task-management-benchmark-api

# Switch to non-root user
USER appuser

# Set environment defaults
ENV HOST=0.0.0.0
ENV PORT=3000
ENV RUST_LOG=info,task_management_benchmark_api=debug

# Expose the API port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Run the API
CMD ["task-management-benchmark-api"]
