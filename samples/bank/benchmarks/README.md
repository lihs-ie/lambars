# Bank Sample API Benchmarks

[日本語](README.ja.md)

This directory contains benchmark scripts for evaluating the performance of the Bank Sample API.

## Prerequisites

- [wrk](https://github.com/wg/wrk) - HTTP benchmarking tool
- Docker environment running (`docker compose up -d`)

### Installing wrk

```bash
# macOS
brew install wrk

# Linux (Ubuntu/Debian)
apt install wrk

# Linux (from source)
git clone https://github.com/wg/wrk.git && cd wrk && make
```

## Quick Start

```bash
# Start the Docker environment
cd ../docker
docker compose up -d

# Run benchmarks (default: 4 threads, 100 connections, 30s duration)
cd ../benchmarks
./run_benchmark.sh

# Run with custom settings
./run_benchmark.sh -t 8 -c 200 -d 60
```

## Benchmark Options

| Option | Default | Description |
|--------|---------|-------------|
| `-t, --threads` | 4 | Number of threads |
| `-c, --connections` | 100 | Number of connections |
| `-d, --duration` | 30 | Duration in seconds |

## Benchmark Targets

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check (baseline) |
| `POST /accounts/{id}/deposit` | Deposit (traditional style) |
| `POST /accounts/{id}/deposit-eff` | Deposit (eff_async! style) |
| `POST /accounts/{id}/withdraw` | Withdraw (traditional style) |
| `POST /accounts/{id}/withdraw-eff` | Withdraw (eff_async! style) |
| `POST /accounts/{id}/transfer` | Transfer between accounts |

## Sample Results

Environment: Apple M3 Pro, Docker Desktop 4.37, 2 CPU cores / 1GB memory limit

### Throughput Comparison

| Endpoint | Requests/sec | Avg Latency | p99 Latency |
|----------|--------------|-------------|-------------|
| Health Check (baseline) | ~41,000 | 5.08ms | 83.14ms |
| Deposit (traditional) | ~5,000 | 20.11ms | 49.23ms |
| Deposit (eff_async!) | ~1,500 | 87.28ms | 689.98ms |
| Withdraw (traditional) | ~1,100 | 89.05ms | 175.51ms |
| Withdraw (eff_async!) | ~850 | 144.71ms | 997.59ms |
| Transfer | ~970 | 102.61ms | 188.92ms |

### Resource Usage

| Metric | Value |
|--------|-------|
| Max CPU | 87% |
| Max Memory | ~1% of 1GB limit |

### Traditional vs eff_async! Style

The `eff_async!` style endpoints show higher latency compared to traditional `?` operator style:

- **Deposit**: Traditional is ~3.4x faster
- **Withdraw**: Traditional is ~1.3x faster

This overhead comes from the monad transformer stack (`ExceptT<ApiError, AsyncIO<...>>`) and the additional abstraction layers. For performance-critical paths, the traditional style with `?` operator is recommended.

## Understanding the Results

### Why are transaction endpoints slower than health check?

1. **Database operations**: Each transaction involves PostgreSQL event store operations
2. **Event sourcing**: Reading and writing event streams
3. **Concurrency**: Lock contention on the same account

### Why are eff_async! endpoints slower?

1. **Monad transformer overhead**: `ExceptT` wrapping adds function call overhead
2. **Boxing**: Effect composition requires heap allocation
3. **Abstraction cost**: Clean functional composition has runtime cost

## Output Files

Results are saved to the `results/` directory:

- `benchmark_YYYYMMDD_HHMMSS.txt` - Detailed benchmark output
- `resources_YYYYMMDD_HHMMSS.csv` - Resource usage time series

## Manual Testing

You can run individual scripts manually:

```bash
# Health check
wrk -t4 -c100 -d10s -s scripts/health.lua http://localhost:8081

# Deposit (requires account ID)
wrk -t4 -c100 -d10s -s scripts/deposit.lua http://localhost:8081 -- <account_id>

# Deposit with eff_async! endpoint
wrk -t4 -c100 -d10s -s scripts/deposit.lua http://localhost:8081 -- <account_id> eff

# Transfer (requires two account IDs)
wrk -t4 -c100 -d10s -s scripts/transfer.lua http://localhost:8081 -- <from_id> <to_id>
```

## Resource Monitoring

While benchmarks run, you can monitor resources separately:

```bash
# Real-time monitoring
docker stats bank-app

# With custom format
docker stats bank-app --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}"
```
