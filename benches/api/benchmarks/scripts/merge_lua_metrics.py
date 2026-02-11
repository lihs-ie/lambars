#!/usr/bin/env python3
"""
Merge lua_metrics.json files from multiple benchmark phases.

Usage:
    merge_lua_metrics.py --output <output_file> <input_file1> [input_file2 ...]
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List


def merge_http_status(files: List[Path], valid_files_set: set) -> Dict[str, int]:
    """Merge http_status counts from multiple lua_metrics.json files."""
    merged = {}

    for file in files:
        if file not in valid_files_set:
            continue

        try:
            with open(file, "r") as f:
                data = json.load(f)
                http_status = data.get("http_status", {})

                if not isinstance(http_status, dict):
                    print(f"Warning: Invalid http_status in {file}, skipping", file=sys.stderr)
                    continue

                for code, count in http_status.items():
                    if not isinstance(count, (int, float)):
                        print(f"Warning: Invalid count for status {code} in {file}, skipping", file=sys.stderr)
                        continue
                    merged[code] = merged.get(code, 0) + int(count)

        except (FileNotFoundError, json.JSONDecodeError) as e:
            print(f"Warning: Failed to process {file}: {e}", file=sys.stderr)

    return merged


def merge_latency_weighted(files: List[Path], valid_requests: Dict[Path, int]) -> Dict[str, Any]:
    """Merge latency metrics using weighted average for all percentiles."""
    total_requests = 0
    weighted = {"mean": 0.0, "p50": 0.0, "p75": 0.0, "p90": 0.0, "p95": 0.0, "p99": 0.0, "p999": 0.0}
    min_ms = float('inf')
    max_ms = 0.0

    for file in files:
        if file not in valid_requests:
            continue

        try:
            with open(file, "r") as f:
                data = json.load(f)
                latency = data.get("latency", {})
                if not isinstance(latency, dict):
                    continue

                requests = valid_requests[file]
                total_requests += requests

                for key in ["mean_ms", "p50_ms", "p75_ms", "p90_ms", "p95_ms", "p99_ms", "p999_ms"]:
                    value = latency.get(key)
                    if isinstance(value, (int, float)):
                        metric_key = key.replace("_ms", "").replace("mean", "mean")
                        weighted[metric_key] += value * requests

                phase_min = latency.get("min_ms", 0)
                if isinstance(phase_min, (int, float)) and phase_min > 0:
                    min_ms = min(min_ms, phase_min)

                phase_max = latency.get("max_ms", 0)
                if isinstance(phase_max, (int, float)):
                    max_ms = max(max_ms, phase_max)

        except (FileNotFoundError, json.JSONDecodeError) as e:
            print(f"Warning: Failed to process {file}: {e}", file=sys.stderr)

    if total_requests == 0:
        return {}

    result = {
        "min_ms": min_ms if min_ms != float('inf') else 0,
        "max_ms": max_ms,
        "mean_ms": weighted["mean"] / total_requests,
    }

    for key, value in weighted.items():
        if key != "mean" and value > 0:
            result[f"{key}_ms"] = value / total_requests

    return result


def validate_request_count(requests: Any, file: Path) -> int:
    """Validate and return request count, or -1 if invalid."""
    if isinstance(requests, bool) or not isinstance(requests, (int, float)):
        print(f"Warning: Invalid or missing total_requests in {file}, skipping", file=sys.stderr)
        return -1
    if isinstance(requests, float) and not requests.is_integer():
        print(f"Warning: Non-integer total_requests ({requests}) in {file}, skipping", file=sys.stderr)
        return -1
    requests = int(requests)
    if requests <= 0:
        print(f"Warning: Non-positive total_requests ({requests}) in {file}, skipping", file=sys.stderr)
        return -1
    return requests

def validate_http_status(http_status: Any, file: Path) -> int:
    """Validate http_status and return sum, or -1 if invalid."""
    if not isinstance(http_status, dict):
        print(f"Warning: Invalid or missing http_status in {file}, skipping", file=sys.stderr)
        return -1

    http_status_sum = 0
    for code, count in http_status.items():
        if isinstance(count, bool) or not isinstance(count, (int, float)):
            print(f"Warning: Non-numeric http_status count for {code} in {file}, skipping file", file=sys.stderr)
            return -1
        if isinstance(count, float) and not count.is_integer():
            print(f"Warning: Non-integer http_status count for {code} ({count}) in {file}, skipping file", file=sys.stderr)
            return -1
        count_int = int(count)
        if count_int < 0:
            print(f"Warning: Negative http_status count for {code} ({count_int}) in {file}, skipping file", file=sys.stderr)
            return -1
        http_status_sum += count_int

    return http_status_sum

def merge_lua_metrics(input_files: List[Path], output_file: Path) -> None:
    """Merge multiple lua_metrics.json files into a single unified file."""
    if not input_files:
        print("Error: No input files provided", file=sys.stderr)
        sys.exit(1)

    total_requests_sum = 0
    valid_requests: Dict[Path, int] = {}
    conflict_detail = {
        "stale_version": 0,
        "retryable_cas": 0,
        "retry_success": 0,
        "retry_exhausted": 0
    }

    for file in input_files:
        try:
            with open(file, "r") as f:
                data = json.load(f)

                requests = validate_request_count(data.get("total_requests", 0), file)
                if requests < 0:
                    continue

                http_status_sum = validate_http_status(data.get("http_status", {}), file)
                if http_status_sum < 0:
                    continue

                if http_status_sum == 0:
                    print(f"Warning: Empty http_status in {file}, excluding from request counts (latency will still be merged)", file=sys.stderr)
                    continue

                if http_status_sum != requests:
                    print(f"Info: http_status sum ({http_status_sum}) != total_requests ({requests}) in {file}, using http_status sum", file=sys.stderr)
                    requests = http_status_sum

                total_requests_sum += requests
                valid_requests[file] = requests

                cd = data.get("conflict_detail", {})
                if isinstance(cd, dict):
                    for key in conflict_detail:
                        value = cd.get(key, 0)
                        if isinstance(value, (int, float)):
                            conflict_detail[key] += int(value)

        except (FileNotFoundError, json.JSONDecodeError) as e:
            print(f"Warning: Failed to read {file}: {e}", file=sys.stderr)

    if not valid_requests:
        print("Error: No valid files to merge", file=sys.stderr)
        sys.exit(1)

    valid_files_set = set(valid_requests.keys())
    http_status = merge_http_status(input_files, valid_files_set)
    latency = merge_latency_weighted(list(valid_requests.keys()), valid_requests)

    http_4xx = sum(count for code, count in http_status.items()
                   if isinstance(count, (int, float)) and code.isdigit() and 400 <= int(code) < 500)
    http_5xx = sum(count for code, count in http_status.items()
                   if isinstance(count, (int, float)) and code.isdigit() and 500 <= int(code) < 600)
    total_errors = http_4xx + http_5xx
    # error_rate = (4xx + 5xx) / total_requests
    # Note: socket_errors (connect/timeout/read/write) are added by shell layer (run_benchmark.sh)
    # and not included in this Lua-level calculation
    error_rate = total_errors / total_requests_sum if total_requests_sum > 0 else 0
    status_distribution = {code: count / total_requests_sum for code, count in http_status.items()
                           if isinstance(count, (int, float))} if total_requests_sum > 0 else {}

    scenario = {}
    execution = {}
    try:
        with open(input_files[0], "r") as f:
            data = json.load(f)
            scenario = data.get("scenario", {})
            execution = data.get("execution", {})
    except (FileNotFoundError, json.JSONDecodeError) as e:
        print(f"Warning: Failed to read metadata from {input_files[0]}: {e}", file=sys.stderr)

    merged = {
        "scenario": scenario,
        "execution": execution,
        "total_requests": total_requests_sum,
        "error_rate": error_rate,
        "http_status": http_status,
        "http_4xx": http_4xx,
        "http_5xx": http_5xx,
        "status_distribution": status_distribution,
        "latency": latency,
        "conflict_detail": conflict_detail,
    }

    output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(output_file, "w") as f:
        json.dump(merged, f, indent=2)

    print(f"Merged {len(input_files)} lua_metrics.json files into {output_file}")


def main():
    parser = argparse.ArgumentParser(
        description="Merge lua_metrics.json files from multiple benchmark phases"
    )
    parser.add_argument(
        "--output",
        type=Path,
        required=True,
        help="Output file path for merged lua_metrics.json",
    )
    parser.add_argument(
        "input_files",
        type=Path,
        nargs="+",
        help="Input lua_metrics.json files to merge",
    )

    args = parser.parse_args()

    # Validate input files
    valid_files = []
    for file in args.input_files:
        if not file.exists():
            print(f"Warning: File not found: {file}", file=sys.stderr)
            continue
        valid_files.append(file)

    if not valid_files:
        print("Error: No valid input files found", file=sys.stderr)
        sys.exit(1)

    merge_lua_metrics(valid_files, args.output)


if __name__ == "__main__":
    main()
