# Profiling Results

This directory contains profiling results from benchmark runs with perf and flamegraph integration.

## Directory Structure

```
profiling-results/
├── README.md                    # This file
├── .gitignore                   # Excludes large binary files
└── {scenario_name}/
    └── {timestamp}/
        ├── metadata.json        # Execution environment information
        ├── perf.data            # perf record data (gitignored)
        ├── perf-report.txt      # perf report output
        ├── flamegraph.svg       # Flamegraph visualization
        ├── wrk-output.json      # wrk benchmark results
        ├── wrk-raw-output.txt   # Raw wrk console output
        └── summary.json         # Combined summary
```

## File Descriptions

### metadata.json

Contains execution environment information:

```json
{
    "scenario_name": "profiling_baseline",
    "timestamp": "2026-01-23T12:00:00Z",
    "platform": "Darwin",
    "platform_version": "23.4.0",
    "hostname": "benchmark-host",
    "cpu_info": "Apple M2",
    "profiling": {
        "frequency_hz": 99,
        "duration_seconds": 30,
        "perf_enabled": true,
        "flamegraph_enabled": true
    },
    "benchmark": {
        "threads": 2,
        "connections": 10,
        "api_url": "http://localhost:3002"
    }
}
```

### wrk-output.json

Extended benchmark results format:

```json
{
    "scenario": {
        "name": "profiling_baseline",
        "storage_mode": "in_memory",
        "cache_mode": "in_memory",
        "load_pattern": "mixed",
        "contention_level": "low"
    },
    "execution": {
        "timestamp": "2026-01-23T12:00:00Z",
        "duration_seconds": 30,
        "threads": 2,
        "connections": 10
    },
    "latency": {
        "min_us": 100,
        "max_us": 50000,
        "mean_us": 1500,
        "stdev_us": 500,
        "percentiles": {
            "p50": 1200,
            "p75": 1800,
            "p90": 2500,
            "p95": 3500,
            "p99": 8000,
            "p99_9": 25000
        }
    },
    "throughput": {
        "requests_total": 150000,
        "requests_per_second": 2500,
        "bytes_total": 45000000,
        "bytes_per_second": 750000
    },
    "errors": {
        "connect": 0,
        "read": 0,
        "write": 0,
        "timeout": 5,
        "status": {
            "4xx": 12,
            "5xx": 3
        }
    },
    "status_distribution": {
        "200": 149980,
        "404": 10,
        "500": 3,
        "503": 7
    }
}
```

### summary.json

Combined summary of all profiling outputs:

```json
{
    "scenario_name": "profiling_baseline",
    "timestamp": "2026-01-23T12:00:00Z",
    "result_directory": "profiling-results/profiling_baseline/20260123_120000",
    "files": {
        "metadata": "metadata.json",
        "wrk_output": "wrk-output.json",
        "perf_data": "perf.data",
        "perf_report": "perf-report.txt",
        "flamegraph": "flamegraph.svg"
    },
    "profiling": {
        "perf_enabled": true,
        "flamegraph_enabled": true,
        "frequency_hz": 99
    }
}
```

## Usage

### Generate profiling results

```bash
# With scenario file
./scripts/profile.sh --flamegraph scenarios/profiling_baseline.yaml

# With command line options
./scripts/profile.sh --flamegraph --duration 60 --frequency 99

# Perf recording only (no flamegraph)
./scripts/profile.sh --perf-record scenarios/profiling_baseline.yaml

# Generate report from existing perf data
./scripts/profile.sh --perf-report
```

### View results

```bash
# Open flamegraph in browser
open profiling-results/profiling_baseline/20260123_120000/flamegraph.svg

# View perf report
less profiling-results/profiling_baseline/20260123_120000/perf-report.txt

# Parse JSON results
jq '.' profiling-results/profiling_baseline/20260123_120000/wrk-output.json
```

## Requirements

- **perf** (Linux): `apt-get install linux-tools-common linux-tools-$(uname -r)`
- **sample** (macOS): Built-in with Xcode Command Line Tools
- **FlameGraph**: Clone from https://github.com/brendangregg/FlameGraph
- **wrk**: `brew install wrk` or `apt-get install wrk`
- **yq**: `brew install yq` or `snap install yq`

## Notes

- Large binary files (perf.data) are excluded from git via .gitignore
- Flamegraph SVG files can be viewed in any web browser
- Results are organized by scenario name and timestamp for easy comparison
