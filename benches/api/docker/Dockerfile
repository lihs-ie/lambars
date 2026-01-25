# syntax=docker/dockerfile:1.4
# Dockerfile for Task Management Benchmark API
#
# Multi-stage build optimized for CI environments.
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

# Copy Cargo files first for better caching
COPY Cargo.toml Cargo.lock ./
COPY lambars-derive/Cargo.toml lambars-derive/Cargo.toml
COPY benches/api/Cargo.toml benches/api/Cargo.toml

# Create directories for Cargo
RUN mkdir -p src lambars-derive/src benches/api/src

# Create dummy bench files to satisfy Cargo.toml bench declarations
RUN mkdir -p benches/iai && \
    for bench in persistent_vector_bench persistent_hashmap_bench persistent_treemap_bench \
                 persistent_list_bench control_bench effect_bench for_macro_bench \
                 for_async_macro_bench algebraic_effect_bench serde_bench transient_bench \
                 rayon_bench freer_bench bifunctor_bench alternative_bench \
                 eff_async_macro_bench async_io_runtime_bench; do \
        echo "fn main() {}" > benches/$bench.rs; \
    done && \
    for bench in persistent_vector_iai effect_iai scenario_iai; do \
        echo "fn main() {}" > benches/iai/$bench.rs; \
    done

# Copy all source files
COPY src/ src/
COPY lambars-derive/src/ lambars-derive/src/
COPY benches/api/src/ benches/api/src/

# Build the binary with cargo registry cache
# Use CARGO_TARGET_DIR to ensure consistent output location
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    CARGO_TARGET_DIR=/build/target cargo build --release \
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
COPY --from=builder /build/target/release/task-management-benchmark-api /usr/local/bin/task-management-benchmark-api

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
