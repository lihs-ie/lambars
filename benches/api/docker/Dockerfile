# syntax=docker/dockerfile:1.4
# Dockerfile for Task Management Benchmark API
#
# Multi-stage build with BuildKit cache optimization.
#
# Build stages:
#   1. builder: Compile the Rust application with cached dependencies
#   2. runtime: Minimal runtime image with only the binary
#
# Usage:
#   docker compose build api
#   docker compose up -d
#
# For CI with GitHub Actions cache:
#   docker buildx build --cache-from type=gha --cache-to type=gha,mode=max .

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

# Step 1: Copy only Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY lambars-derive/Cargo.toml lambars-derive/Cargo.toml
COPY benches/api/Cargo.toml benches/api/Cargo.toml

# Step 2: Create dummy source files to build dependencies
# Also create dummy bench files to satisfy Cargo.toml bench declarations
RUN mkdir -p src lambars-derive/src benches/api/src benches/iai && \
    echo "fn main() {}" > benches/api/src/main.rs && \
    echo "" > src/lib.rs && \
    echo "" > lambars-derive/src/lib.rs && \
    for bench in persistent_vector_bench persistent_hashmap_bench persistent_treemap_bench \
                 persistent_list_bench control_bench effect_bench for_macro_bench \
                 for_async_macro_bench algebraic_effect_bench serde_bench transient_bench \
                 rayon_bench freer_bench bifunctor_bench alternative_bench; do \
        echo "fn main() {}" > benches/$bench.rs; \
    done && \
    for bench in persistent_vector_iai effect_iai scenario_iai; do \
        echo "fn main() {}" > benches/iai/$bench.rs; \
    done

# Step 3: Build dependencies only with cache mount
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release \
    --manifest-path benches/api/Cargo.toml \
    --bin task-management-benchmark-api 2>/dev/null || true

# Step 4: Remove dummy files and copy real source
RUN rm -rf src lambars-derive/src benches/api/src
COPY src/ src/
COPY lambars-derive/src/ lambars-derive/src/
COPY benches/api/src/ benches/api/src/

# Step 5: Build the actual binary with cache mount
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --release \
    --manifest-path benches/api/Cargo.toml \
    --bin task-management-benchmark-api && \
    cp /build/target/release/task-management-benchmark-api /tmp/app

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
COPY --from=builder /tmp/app /usr/local/bin/task-management-benchmark-api

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
