# Build stage
FROM rust:1.92-bookworm AS builder

WORKDIR /build

# Copy lambars library (full workspace needed for build)
COPY Cargo.toml Cargo.lock /build/
COPY lambars-derive /build/lambars-derive
COPY src /build/src
COPY benches /build/benches
COPY tests /build/tests

# Copy roguelike workspace
COPY samples/roguelike /build/samples/roguelike

WORKDIR /build/samples/roguelike

# Build release binary
RUN cargo build --release -p roguelike-api

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/samples/roguelike/target/release/roguelike-server /app/roguelike-server

EXPOSE 8080

CMD ["/app/roguelike-server"]
