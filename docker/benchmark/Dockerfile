# Dockerfile for Iai-Callgrind benchmarks
#
# This image provides Valgrind/Callgrind + Rust for running
# instruction-count based benchmarks that are deterministic
# and suitable for CI regression detection.
#
# Usage:
#   docker compose run --rm benchmark cargo bench --bench persistent_vector_iai
#
# Requirements:
#   - SYS_PTRACE capability (for Valgrind)
#   - seccomp=unconfined (for ptrace syscall)

FROM rust:1.92.0-bookworm

# Install Valgrind and other required tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    valgrind \
    binutils \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set up working directory
WORKDIR /workspace

# Install iai-callgrind-runner (must match iai-callgrind version in Cargo.toml)
# This is the runner binary that iai-callgrind benchmarks use
RUN cargo install iai-callgrind-runner --version 0.14.0

# Copy the project files (done at build time or via volume mount)
# For development, use volume mounts in compose.yaml

# Note: Running as root is required for Valgrind/Callgrind to trace
# processes properly. This container is intended for local development
# and CI benchmarks only, not production use.

# Default command
CMD ["cargo", "bench", "--help"]
