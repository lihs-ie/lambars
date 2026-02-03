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
    """Merge http_status counts from multiple lua_metrics.json files.

    Only processes files in valid_files_set to ensure consistency with total_requests.
    """
    merged = {}

    for file in files:
        # Skip files that didn't have valid total_requests
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
    """Merge latency metrics using weighted average for all percentiles (REQ-PIPELINE-002).

    Args:
        files: List of lua_metrics.json file paths
        valid_requests: Map of file path to validated request count (from http_status sum)
    """
    total_requests = 0
    weighted_mean = 0.0
    weighted_p50 = 0.0
    weighted_p75 = 0.0
    weighted_p90 = 0.0
    weighted_p95 = 0.0
    weighted_p99 = 0.0
    weighted_p999 = 0.0
    min_ms = float('inf')
    max_ms = 0.0

    for file in files:
        # Skip files not in valid_requests
        if file not in valid_requests:
            continue

        try:
            with open(file, "r") as f:
                data = json.load(f)
                latency = data.get("latency", {})

                # Use validated request count from http_status sum
                requests = valid_requests[file]

                if not isinstance(latency, dict):
                    continue

                total_requests += requests

                # Weight all latency metrics by request count
                mean_ms = latency.get("mean_ms", 0)
                if isinstance(mean_ms, (int, float)):
                    weighted_mean += mean_ms * requests

                p50_ms = latency.get("p50_ms")
                if isinstance(p50_ms, (int, float)):
                    weighted_p50 += p50_ms * requests

                p75_ms = latency.get("p75_ms")
                if isinstance(p75_ms, (int, float)):
                    weighted_p75 += p75_ms * requests

                p90_ms = latency.get("p90_ms")
                if isinstance(p90_ms, (int, float)):
                    weighted_p90 += p90_ms * requests

                p95_ms = latency.get("p95_ms")
                if isinstance(p95_ms, (int, float)):
                    weighted_p95 += p95_ms * requests

                p99_ms = latency.get("p99_ms")
                if isinstance(p99_ms, (int, float)):
                    weighted_p99 += p99_ms * requests

                p999_ms = latency.get("p999_ms")
                if isinstance(p999_ms, (int, float)):
                    weighted_p999 += p999_ms * requests

                # Track min and max across all phases
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

    # Calculate weighted averages
    result = {
        "min_ms": min_ms if min_ms != float('inf') else 0,
        "max_ms": max_ms,
        "mean_ms": weighted_mean / total_requests,
    }

    # Add percentiles if they were present
    if weighted_p50 > 0:
        result["p50_ms"] = weighted_p50 / total_requests
    if weighted_p75 > 0:
        result["p75_ms"] = weighted_p75 / total_requests
    if weighted_p90 > 0:
        result["p90_ms"] = weighted_p90 / total_requests
    if weighted_p95 > 0:
        result["p95_ms"] = weighted_p95 / total_requests
    if weighted_p99 > 0:
        result["p99_ms"] = weighted_p99 / total_requests
    if weighted_p999 > 0:
        result["p999_ms"] = weighted_p999 / total_requests

    # Note: stddev cannot be meaningfully averaged, so we omit it
    # A proper stddev merge would require raw data or variance

    return result


def merge_lua_metrics(input_files: List[Path], output_file: Path) -> None:
    """Merge multiple lua_metrics.json files into a single unified file."""
    if not input_files:
        print("Error: No input files provided", file=sys.stderr)
        sys.exit(1)

    # Validate each file for both total_requests and http_status (REQ-PIPELINE-002)
    # Track validated request counts per file for consistent latency weighting
    total_requests_sum = 0
    valid_requests: Dict[Path, int] = {}  # file -> validated request count
    for file in input_files:
        try:
            with open(file, "r") as f:
                data = json.load(f)

                # Check total_requests (must be a positive integer, not bool)
                requests = data.get("total_requests", 0)
                if isinstance(requests, bool) or not isinstance(requests, (int, float)):
                    print(f"Warning: Invalid or missing total_requests in {file}, skipping", file=sys.stderr)
                    continue
                if isinstance(requests, float) and not requests.is_integer():
                    print(f"Warning: Non-integer total_requests ({requests}) in {file}, skipping", file=sys.stderr)
                    continue
                requests = int(requests)
                if requests <= 0:
                    print(f"Warning: Non-positive total_requests ({requests}) in {file}, skipping", file=sys.stderr)
                    continue

                # Check http_status and validate consistency
                http_status = data.get("http_status", {})
                if not isinstance(http_status, dict):
                    print(f"Warning: Invalid or missing http_status in {file}, skipping", file=sys.stderr)
                    continue

                # Sum http_status counts and validate all values are non-negative integers (not bool)
                http_status_sum = 0
                has_invalid_count = False
                for code, count in http_status.items():
                    if isinstance(count, bool) or not isinstance(count, (int, float)):
                        print(f"Warning: Non-numeric http_status count for {code} in {file}, skipping file", file=sys.stderr)
                        has_invalid_count = True
                        break
                    if isinstance(count, float) and not count.is_integer():
                        print(f"Warning: Non-integer http_status count for {code} ({count}) in {file}, skipping file", file=sys.stderr)
                        has_invalid_count = True
                        break
                    count_int = int(count)
                    if count_int < 0:
                        print(f"Warning: Negative http_status count for {code} ({count_int}) in {file}, skipping file", file=sys.stderr)
                        has_invalid_count = True
                        break
                    http_status_sum += count_int

                if has_invalid_count:
                    continue

                # http_status must be non-empty
                if http_status_sum == 0:
                    print(f"Warning: Empty http_status in {file}, skipping", file=sys.stderr)
                    continue

                # Validation: http_status sum should equal total_requests
                # If mismatch, use http_status sum as authoritative (thread-local count)
                # This handles wrk's thread isolation where summary.requests may differ
                if http_status_sum != requests:
                    print(f"Info: http_status sum ({http_status_sum}) != total_requests ({requests}) in {file}, using http_status sum", file=sys.stderr)
                    # Use http_status sum as the authoritative request count
                    requests = http_status_sum

                # File is valid - add to tracking with validated request count
                total_requests_sum += requests
                valid_requests[file] = requests

        except (FileNotFoundError, json.JSONDecodeError) as e:
            print(f"Warning: Failed to read {file}: {e}", file=sys.stderr)

    if not valid_requests:
        print("Error: No valid files to merge", file=sys.stderr)
        sys.exit(1)

    # Merge http_status and latency only from valid files
    # This ensures all metrics are consistent with total_requests
    valid_files_set = set(valid_requests.keys())
    http_status = merge_http_status(input_files, valid_files_set)
    latency = merge_latency_weighted(list(valid_requests.keys()), valid_requests)

    # Calculate error metrics using actual total_requests
    total_errors = sum(
        count for code_str, count in http_status.items()
        if isinstance(count, (int, float)) and code_str.isdigit() and 400 <= int(code_str) < 600
    )
    error_rate = total_errors / total_requests_sum if total_requests_sum > 0 else 0
    status_distribution = {
        code: count / total_requests_sum
        for code, count in http_status.items()
        if isinstance(count, (int, float))
    } if total_requests_sum > 0 else {}

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
        "status_distribution": status_distribution,
        "latency": latency,
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
