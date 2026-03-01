FROM rust:1.92-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy lambars library (from workspace root context)
COPY Cargo.toml ./lambars/Cargo.toml
COPY src ./lambars/src
COPY lambars-derive ./lambars/lambars-derive

# Copy bank sample
COPY samples/bank/Cargo.toml samples/bank/Cargo.lock ./
COPY samples/bank/src ./src
COPY samples/bank/migrations ./migrations

# Update path references for Docker context
RUN sed -i 's|path = "../.."|path = "./lambars"|g' Cargo.toml && \
    sed -i 's|path = "../../lambars-derive"|path = "./lambars/lambars-derive"|g' Cargo.toml

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/bank /app/bank
COPY --from=builder /app/migrations /app/migrations

ENV RUST_LOG=info,bank=debug

EXPOSE 8080

CMD ["/app/bank"]
