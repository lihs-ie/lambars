#!/bin/bash
# benches/api/benchmarks/compare_results.sh
#
# Compare two benchmark result directories and report differences.
#
# Usage:
#   ./compare_results.sh <base_dir> <new_dir> [--json] [--threshold <file>]
#   ./compare_results.sh results/20260122_120000 results/20260123_140000
#   ./compare_results.sh results/20260122_120000 results/20260123_140000 --json
#   ./compare_results.sh base new --threshold thresholds.yaml
#
# Options:
#   --json              Output results as JSON (for CI integration)
#   --threshold <file>  Path to threshold YAML file for regression detection
#
# Output:
#   - Comparison table with metrics: p50, p90, p99, rps, error_rate
#   - Percentage differences (positive = improvement, negative = regression)
#   - Memory metrics if profile data available
#
# Exit codes:
#   0 - Success, no significant regression
#   1 - Missing arguments or directories
#   2 - No comparable results found
#   3 - Regression detected (when using thresholds)

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# =============================================================================
# Argument Parsing
# =============================================================================

JSON_OUTPUT=false
THRESHOLD_FILE=""

show_usage() {
    echo "Usage: $0 <base_dir> <new_dir> [--json] [--threshold <file>]"
    echo ""
    echo "Options:"
    echo "  --json              Output results as JSON"
    echo "  --threshold <file>  Path to threshold YAML file"
    echo ""
    echo "Examples:"
    echo "  $0 results/20260122_120000 results/20260123_140000"
    echo "  $0 results/baseline results/optimized --json"
    exit 1
}

if [[ $# -lt 2 ]]; then
    show_usage
fi

BASE_DIR="$1"
NEW_DIR="$2"
shift 2

while [[ $# -gt 0 ]]; do
    case "$1" in
        --json)
            JSON_OUTPUT=true
            shift
            ;;
        --threshold)
            THRESHOLD_FILE="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            show_usage
            ;;
    esac
done

if [[ ! -d "${BASE_DIR}" ]]; then
    echo -e "${RED}Error: Base directory not found: ${BASE_DIR}${NC}"
    exit 1
fi

if [[ ! -d "${NEW_DIR}" ]]; then
    echo -e "${RED}Error: New directory not found: ${NEW_DIR}${NC}"
    exit 1
fi

# =============================================================================
# JSON Output and Threshold Storage
# =============================================================================

# Array to store comparison results for JSON output
declare -a JSON_RESULTS=()
REGRESSION_DETECTED=false

# Load threshold values from YAML file
load_thresholds() {
    local file="$1"
    if [[ ! -f "${file}" ]]; then
        echo -e "${RED}Error: Threshold file not found: ${file}${NC}"
        exit 1
    fi

    # Parse thresholds with yq if available, otherwise use grep/sed
    if command -v yq &> /dev/null; then
        P90_WARN=$(yq '.thresholds.p90_latency_ms.warning // 50' "${file}")
        P90_ERROR=$(yq '.thresholds.p90_latency_ms.error // 100' "${file}")
        P99_WARN=$(yq '.thresholds.p99_latency_ms.warning // 100' "${file}")
        P99_ERROR=$(yq '.thresholds.p99_latency_ms.error // 200' "${file}")
        ERROR_RATE_WARN=$(yq '.thresholds.error_rate.warning // 0.01' "${file}")
        ERROR_RATE_ERROR=$(yq '.thresholds.error_rate.error // 0.05' "${file}")
        RPS_DEGRADATION_WARN=$(yq '.thresholds.rps_degradation_percent.warning // 5' "${file}")
        RPS_DEGRADATION_ERROR=$(yq '.thresholds.rps_degradation_percent.error // 10' "${file}")
    else
        # Fallback to grep/sed parsing
        P90_WARN=$(grep -A2 'p90_latency_ms:' "${file}" 2>/dev/null | grep 'warning:' | sed 's/.*warning: *//' || echo "50")
        P90_ERROR=$(grep -A2 'p90_latency_ms:' "${file}" 2>/dev/null | grep 'error:' | sed 's/.*error: *//' || echo "100")
        P99_WARN=$(grep -A2 'p99_latency_ms:' "${file}" 2>/dev/null | grep 'warning:' | sed 's/.*warning: *//' || echo "100")
        P99_ERROR=$(grep -A2 'p99_latency_ms:' "${file}" 2>/dev/null | grep 'error:' | sed 's/.*error: *//' || echo "200")
        ERROR_RATE_WARN=$(grep -A2 'error_rate:' "${file}" 2>/dev/null | grep 'warning:' | sed 's/.*warning: *//' || echo "0.01")
        ERROR_RATE_ERROR=$(grep -A2 'error_rate:' "${file}" 2>/dev/null | grep 'error:' | sed 's/.*error: *//' || echo "0.05")
        RPS_DEGRADATION_WARN=$(grep -A2 'rps_degradation_percent:' "${file}" 2>/dev/null | grep 'warning:' | sed 's/.*warning: *//' || echo "5")
        RPS_DEGRADATION_ERROR=$(grep -A2 'rps_degradation_percent:' "${file}" 2>/dev/null | grep 'error:' | sed 's/.*error: *//' || echo "10")
    fi
}

# Check if a metric exceeds threshold
# Returns: "ok" | "warning" | "error"
check_threshold() {
    local value="$1"
    local warn_threshold="$2"
    local error_threshold="$3"
    local higher_is_worse="${4:-true}"

    if [[ "${value}" == "N/A" ]] || [[ -z "${value}" ]]; then
        echo "ok"
        return
    fi

    if [[ "${higher_is_worse}" == "true" ]]; then
        if (( $(echo "${value} >= ${error_threshold}" | bc -l 2>/dev/null || echo "0") )); then
            echo "error"
        elif (( $(echo "${value} >= ${warn_threshold}" | bc -l 2>/dev/null || echo "0") )); then
            echo "warning"
        else
            echo "ok"
        fi
    else
        # For metrics where lower is worse (like RPS degradation as negative)
        local abs_value
        abs_value=$(echo "${value}" | sed 's/^-//')
        if (( $(echo "${abs_value} >= ${error_threshold}" | bc -l 2>/dev/null || echo "0") )); then
            echo "error"
        elif (( $(echo "${abs_value} >= ${warn_threshold}" | bc -l 2>/dev/null || echo "0") )); then
            echo "warning"
        else
            echo "ok"
        fi
    fi
}

# =============================================================================
# Utility Functions
# =============================================================================

# Convert value to JSON-safe format
# - "N/A" or empty → null
# - numeric value → as-is (unquoted)
# - string → quoted
json_value() {
    local value="$1"
    local as_string="${2:-false}"  # if true, output as quoted string

    if [[ -z "${value}" ]] || [[ "${value}" == "N/A" ]] || [[ "${value}" == "null" ]]; then
        echo "null"
    elif [[ "${as_string}" == "true" ]]; then
        # Escape backslashes and double quotes for JSON
        local escaped
        escaped=$(echo "${value}" | sed 's/\\/\\\\/g; s/"/\\"/g')
        echo "\"${escaped}\""
    else
        echo "${value}"
    fi
}

# Parse latency value and convert to milliseconds
# Handles: 1.23ms, 123.45us, 1.5s
parse_latency_ms() {
    local value="$1"
    if [[ -z "${value}" ]] || [[ "${value}" == "N/A" ]]; then
        echo "0"
        return
    fi

    local num unit
    num=$(echo "${value}" | sed 's/[^0-9.]//g')
    unit=$(echo "${value}" | sed 's/[0-9.]//g')

    case "${unit}" in
        us) echo "scale=4; ${num} / 1000" | bc ;;
        ms) echo "${num}" ;;
        s)  echo "scale=4; ${num} * 1000" | bc ;;
        *)  echo "${num}" ;;
    esac
}

# Calculate percentage difference
# Returns: positive = improvement (new is better), negative = regression
calc_diff_percent() {
    local base="$1"
    local new="$2"
    local metric_type="${3:-latency}"  # latency or throughput

    # Handle N/A and empty values
    if [[ -z "${base}" ]] || [[ "${base}" == "0" ]] || [[ "${base}" == "N/A" ]]; then
        echo "N/A"
        return
    fi

    if [[ -z "${new}" ]] || [[ "${new}" == "N/A" ]]; then
        echo "N/A"
        return
    fi

    local diff
    if [[ "${metric_type}" == "latency" ]]; then
        # For latency, lower is better: positive diff = improvement
        diff=$(echo "scale=2; ((${base} - ${new}) / ${base}) * 100" | bc 2>/dev/null || echo "N/A")
    else
        # For throughput, higher is better: positive diff = improvement
        diff=$(echo "scale=2; ((${new} - ${base}) / ${base}) * 100" | bc 2>/dev/null || echo "N/A")
    fi

    echo "${diff}"
}

# Format difference with color
format_diff() {
    local diff="$1"
    local metric_type="${2:-latency}"

    if [[ "${diff}" == "N/A" ]]; then
        echo "N/A"
        return
    fi

    local sign=""
    local color="${NC}"

    # Check if positive or negative
    if (( $(echo "${diff} > 0" | bc -l 2>/dev/null || echo "0") )); then
        sign="+"
        color="${GREEN}"
    elif (( $(echo "${diff} < 0" | bc -l 2>/dev/null || echo "0") )); then
        color="${RED}"
    fi

    echo -e "${color}${sign}${diff}%${NC}"
}

# Extract metric from wrk.txt
extract_from_wrk() {
    local file="$1"
    local pattern="$2"

    if [[ ! -f "${file}" ]]; then
        echo "N/A"
        return
    fi

    grep "${pattern}" "${file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "N/A"
}

# Extract metric from meta.json
extract_from_meta() {
    local file="$1"
    local key="$2"

    if [[ ! -f "${file}" ]]; then
        echo "N/A"
        return
    fi

    # Use jq if available, otherwise use grep/sed
    if command -v jq &> /dev/null; then
        jq -r ".results.${key} // \"N/A\"" "${file}" 2>/dev/null || echo "N/A"
    else
        grep "\"${key}\"" "${file}" 2>/dev/null | head -1 | sed 's/.*: *"\?\([^",}]*\)"\?.*/\1/' || echo "N/A"
    fi
}

# Extract metric from wrk-output.json
extract_from_wrk_json() {
    local file="$1"
    local key="$2"

    if [[ ! -f "${file}" ]]; then
        echo "N/A"
        return
    fi

    # Use jq if available, otherwise use grep/sed
    if command -v jq &> /dev/null; then
        local value
        case "${key}" in
            rps)
                value=$(jq -r '.throughput.requests_per_second // "N/A"' "${file}" 2>/dev/null || echo "N/A")
                ;;
            p50)
                value=$(jq -r '.latency.percentiles.p50 // "N/A"' "${file}" 2>/dev/null || echo "N/A")
                ;;
            p75)
                value=$(jq -r '.latency.percentiles.p75 // "N/A"' "${file}" 2>/dev/null || echo "N/A")
                ;;
            p90)
                value=$(jq -r '.latency.percentiles.p90 // "N/A"' "${file}" 2>/dev/null || echo "N/A")
                ;;
            p99)
                value=$(jq -r '.latency.percentiles.p99 // "N/A"' "${file}" 2>/dev/null || echo "N/A")
                ;;
            avg)
                value=$(jq -r '.latency.mean // "N/A"' "${file}" 2>/dev/null || echo "N/A")
                ;;
            error_rate)
                # Calculate error rate from connection-level errors
                # Note: wrk only reports connection-level errors (connect, read, write, timeout).
                # HTTP 4xx/5xx responses are NOT captured by wrk's error counters.
                # For HTTP status code analysis, use:
                #   - Lua script with response.status tracking (tasks_update.lua)
                #   - result_collector.lua for detailed status distribution
                # This error_rate metric is useful for detecting network/connection issues,
                # but NOT for detecting application-level errors (5xx).
                local connect read write timeout total_requests
                connect=$(jq -r '.errors.connect // 0' "${file}" 2>/dev/null || echo "0")
                read=$(jq -r '.errors.read // 0' "${file}" 2>/dev/null || echo "0")
                write=$(jq -r '.errors.write // 0' "${file}" 2>/dev/null || echo "0")
                timeout=$(jq -r '.errors.timeout // 0' "${file}" 2>/dev/null || echo "0")
                total_requests=$(jq -r '.throughput.requests_total // 0' "${file}" 2>/dev/null || echo "0")

                local total_errors
                total_errors=$((connect + read + write + timeout))

                if [[ ${total_requests} -gt 0 ]]; then
                    value=$(echo "scale=6; ${total_errors} / ${total_requests}" | bc 2>/dev/null || echo "N/A")
                else
                    value="N/A"
                fi
                ;;
            *)
                value="N/A"
                ;;
        esac
        echo "${value}"
    else
        # Fallback to grep/sed parsing
        case "${key}" in
            rps)
                grep '"requests_per_second"' "${file}" 2>/dev/null | head -1 | sed 's/.*: *"\?\([^",}]*\)"\?.*/\1/' || echo "N/A"
                ;;
            p50)
                grep '"p50"' "${file}" 2>/dev/null | head -1 | sed 's/.*: *"\?\([^",}]*\)"\?.*/\1/' || echo "N/A"
                ;;
            p90)
                grep '"p90"' "${file}" 2>/dev/null | head -1 | sed 's/.*: *"\?\([^",}]*\)"\?.*/\1/' || echo "N/A"
                ;;
            p99)
                grep '"p99"' "${file}" 2>/dev/null | head -1 | sed 's/.*: *"\?\([^",}]*\)"\?.*/\1/' || echo "N/A"
                ;;
            avg)
                grep '"mean"' "${file}" 2>/dev/null | head -1 | sed 's/.*: *"\?\([^",}]*\)"\?.*/\1/' || echo "N/A"
                ;;
            *)
                echo "N/A"
                ;;
        esac
    fi
}

# =============================================================================
# Result Collection
# =============================================================================

# Find all result files (wrk.txt, wrk-output.json, or meta.json) in a directory
find_results() {
    local dir="$1"
    local results=()

    # Check if directory has direct wrk.txt or wrk-output.json (single scenario)
    if [[ -f "${dir}/wrk.txt" ]] || [[ -f "${dir}/wrk-output.json" ]]; then
        echo "${dir}"
        return
    fi

    # Check subdirectories
    for subdir in "${dir}"/*/; do
        if [[ -d "${subdir}" ]]; then
            if [[ -f "${subdir}/wrk.txt" ]] || [[ -f "${subdir}/wrk-output.json" ]] || [[ -f "${subdir}/meta.json" ]]; then
                echo "$(basename "${subdir}")"
            fi
        fi
    done
}

# =============================================================================
# Comparison Logic
# =============================================================================

compare_single() {
    local base_path="$1"
    local new_path="$2"
    local scenario_name="$3"

    local base_wrk="${base_path}/wrk.txt"
    local new_wrk="${new_path}/wrk.txt"
    local base_wrk_json="${base_path}/wrk-output.json"
    local new_wrk_json="${new_path}/wrk-output.json"
    local base_meta="${base_path}/meta.json"
    local new_meta="${new_path}/meta.json"

    # Extract metrics from wrk-output.json (preferred) or wrk.txt (fallback)
    local base_rps new_rps
    local base_p50 new_p50
    local base_p90 new_p90
    local base_p99 new_p99
    local base_avg new_avg

    # Base metrics
    if [[ -f "${base_wrk_json}" ]]; then
        base_rps=$(extract_from_wrk_json "${base_wrk_json}" "rps")
        base_p50=$(extract_from_wrk_json "${base_wrk_json}" "p50")
        base_p90=$(extract_from_wrk_json "${base_wrk_json}" "p90")
        base_p99=$(extract_from_wrk_json "${base_wrk_json}" "p99")
        base_avg=$(extract_from_wrk_json "${base_wrk_json}" "avg")
    else
        base_rps=$(extract_from_wrk "${base_wrk}" "Requests/sec:")
        base_p50=$(extract_from_wrk "${base_wrk}" "50%")
        base_p90=$(extract_from_wrk "${base_wrk}" "90%")
        base_p99=$(extract_from_wrk "${base_wrk}" "99%")
        base_avg=$(extract_from_wrk "${base_wrk}" "Latency")
    fi

    # New metrics
    if [[ -f "${new_wrk_json}" ]]; then
        new_rps=$(extract_from_wrk_json "${new_wrk_json}" "rps")
        new_p50=$(extract_from_wrk_json "${new_wrk_json}" "p50")
        new_p90=$(extract_from_wrk_json "${new_wrk_json}" "p90")
        new_p99=$(extract_from_wrk_json "${new_wrk_json}" "p99")
        new_avg=$(extract_from_wrk_json "${new_wrk_json}" "avg")
    else
        new_rps=$(extract_from_wrk "${new_wrk}" "Requests/sec:")
        new_p50=$(extract_from_wrk "${new_wrk}" "50%")
        new_p90=$(extract_from_wrk "${new_wrk}" "90%")
        new_p99=$(extract_from_wrk "${new_wrk}" "99%")
        new_avg=$(extract_from_wrk "${new_wrk}" "Latency")
    fi

    # Extract error rate from wrk-output.json (preferred), meta.json, or wrk.txt
    local base_error new_error
    if [[ -f "${base_wrk_json}" ]]; then
        base_error=$(extract_from_wrk_json "${base_wrk_json}" "error_rate")
    else
        base_error=$(extract_from_meta "${base_meta}" "error_rate")
    fi

    if [[ -f "${new_wrk_json}" ]]; then
        new_error=$(extract_from_wrk_json "${new_wrk_json}" "error_rate")
    else
        new_error=$(extract_from_meta "${new_meta}" "error_rate")
    fi

    # Convert latencies to ms for comparison
    local base_p50_ms new_p50_ms
    local base_p90_ms new_p90_ms
    local base_p99_ms new_p99_ms

    base_p50_ms=$(parse_latency_ms "${base_p50}")
    new_p50_ms=$(parse_latency_ms "${new_p50}")

    base_p90_ms=$(parse_latency_ms "${base_p90}")
    new_p90_ms=$(parse_latency_ms "${new_p90}")

    base_p99_ms=$(parse_latency_ms "${base_p99}")
    new_p99_ms=$(parse_latency_ms "${new_p99}")

    # Calculate differences
    local diff_rps diff_p50 diff_p90 diff_p99 diff_error

    diff_rps=$(calc_diff_percent "${base_rps}" "${new_rps}" "throughput")
    diff_p50=$(calc_diff_percent "${base_p50_ms}" "${new_p50_ms}" "latency")
    diff_p90=$(calc_diff_percent "${base_p90_ms}" "${new_p90_ms}" "latency")
    diff_p99=$(calc_diff_percent "${base_p99_ms}" "${new_p99_ms}" "latency")

    if [[ "${base_error}" != "N/A" ]] && [[ "${new_error}" != "N/A" ]]; then
        diff_error=$(calc_diff_percent "${base_error}" "${new_error}" "latency")
    else
        diff_error="N/A"
    fi

    # Check thresholds if threshold file is specified
    local p90_status="ok" p99_status="ok" rps_status="ok" error_status="ok"
    if [[ -n "${THRESHOLD_FILE}" ]]; then
        p90_status=$(check_threshold "${new_p90_ms}" "${P90_WARN}" "${P90_ERROR}" "true")
        p99_status=$(check_threshold "${new_p99_ms}" "${P99_WARN}" "${P99_ERROR}" "true")

        # RPS degradation: negative diff means regression
        if [[ "${diff_rps}" != "N/A" ]] && (( $(echo "${diff_rps} < 0" | bc -l 2>/dev/null || echo "0") )); then
            local rps_degradation
            rps_degradation=$(echo "${diff_rps}" | sed 's/^-//')
            rps_status=$(check_threshold "${rps_degradation}" "${RPS_DEGRADATION_WARN}" "${RPS_DEGRADATION_ERROR}" "true")
        fi

        if [[ "${new_error}" != "N/A" ]]; then
            error_status=$(check_threshold "${new_error}" "${ERROR_RATE_WARN}" "${ERROR_RATE_ERROR}" "true")
        fi

        # Mark regression if any error threshold exceeded
        if [[ "${p90_status}" == "error" ]] || [[ "${p99_status}" == "error" ]] || \
           [[ "${rps_status}" == "error" ]] || [[ "${error_status}" == "error" ]]; then
            REGRESSION_DETECTED=true
        fi
    fi

    # Store result for JSON output (using json_value to handle N/A → null conversion)
    local json_entry
    json_entry=$(cat <<EOF
{
  "scenario": "${scenario_name}",
  "base": {
    "p50": $(json_value "${base_p50}" true),
    "p90": $(json_value "${base_p90}" true),
    "p99": $(json_value "${base_p99}" true),
    "rps": $(json_value "${base_rps}"),
    "error_rate": $(json_value "${base_error}")
  },
  "new": {
    "p50": $(json_value "${new_p50}" true),
    "p90": $(json_value "${new_p90}" true),
    "p99": $(json_value "${new_p99}" true),
    "rps": $(json_value "${new_rps}"),
    "error_rate": $(json_value "${new_error}")
  },
  "diff": {
    "p50_percent": $(json_value "${diff_p50}"),
    "p90_percent": $(json_value "${diff_p90}"),
    "p99_percent": $(json_value "${diff_p99}"),
    "rps_percent": $(json_value "${diff_rps}"),
    "error_rate_percent": $(json_value "${diff_error}")
  },
  "status": {
    "p90": "${p90_status}",
    "p99": "${p99_status}",
    "rps": "${rps_status}",
    "error_rate": "${error_status}"
  }
}
EOF
)
    JSON_RESULTS+=("${json_entry}")

    # Print comparison table (skip if JSON output mode)
    if [[ "${JSON_OUTPUT}" != "true" ]]; then
        echo ""
        echo -e "${BOLD}${CYAN}=== ${scenario_name} ===${NC}"
        echo ""
        printf "%-12s | %-12s | %-12s | %-10s\n" "Metric" "Base" "New" "Diff"
        printf "%-12s-+-%-12s-+-%-12s-+-%-10s\n" "------------" "------------" "------------" "----------"
        printf "%-12s | %-12s | %-12s | %s\n" "p50" "${base_p50:-N/A}" "${new_p50:-N/A}" "$(format_diff "${diff_p50}" "latency")"
        printf "%-12s | %-12s | %-12s | %s\n" "p90" "${base_p90:-N/A}" "${new_p90:-N/A}" "$(format_diff "${diff_p90}" "latency")"
        printf "%-12s | %-12s | %-12s | %s\n" "p99" "${base_p99:-N/A}" "${new_p99:-N/A}" "$(format_diff "${diff_p99}" "latency")"
        printf "%-12s | %-12s | %-12s | %s\n" "rps" "${base_rps:-N/A}" "${new_rps:-N/A}" "$(format_diff "${diff_rps}" "throughput")"

        if [[ "${base_error}" != "N/A" ]] || [[ "${new_error}" != "N/A" ]]; then
            printf "%-12s | %-12s | %-12s | %s\n" "error_rate" "${base_error:-N/A}" "${new_error:-N/A}" "$(format_diff "${diff_error}" "latency")"
        fi

        # Show threshold warnings/errors
        if [[ -n "${THRESHOLD_FILE}" ]]; then
            if [[ "${p90_status}" != "ok" ]]; then
                local status_color="${YELLOW}"
                [[ "${p90_status}" == "error" ]] && status_color="${RED}"
                echo -e "${status_color}  [${p90_status}] p90 latency: ${new_p90_ms}ms (warn: ${P90_WARN}ms, error: ${P90_ERROR}ms)${NC}"
            fi
            if [[ "${p99_status}" != "ok" ]]; then
                local status_color="${YELLOW}"
                [[ "${p99_status}" == "error" ]] && status_color="${RED}"
                echo -e "${status_color}  [${p99_status}] p99 latency: ${new_p99_ms}ms (warn: ${P99_WARN}ms, error: ${P99_ERROR}ms)${NC}"
            fi
            if [[ "${rps_status}" != "ok" ]]; then
                local status_color="${YELLOW}"
                [[ "${rps_status}" == "error" ]] && status_color="${RED}"
                echo -e "${status_color}  [${rps_status}] RPS degradation: ${diff_rps}% (warn: ${RPS_DEGRADATION_WARN}%, error: ${RPS_DEGRADATION_ERROR}%)${NC}"
            fi
            if [[ "${error_status}" != "ok" ]]; then
                local status_color="${YELLOW}"
                [[ "${error_status}" == "error" ]] && status_color="${RED}"
                echo -e "${status_color}  [${error_status}] Error rate: ${new_error} (warn: ${ERROR_RATE_WARN}, error: ${ERROR_RATE_ERROR})${NC}"
            fi
        fi

        # Check for profiling data
        local base_flamegraph="${base_path}/flamegraph.svg"
        local new_flamegraph="${new_path}/flamegraph.svg"

        if [[ -f "${base_flamegraph}" ]] || [[ -f "${new_flamegraph}" ]]; then
            echo ""
            echo -e "${YELLOW}Profiling data available:${NC}"
            [[ -f "${base_flamegraph}" ]] && echo "  Base: ${base_flamegraph}"
            [[ -f "${new_flamegraph}" ]] && echo "  New:  ${new_flamegraph}"
        fi
    fi
}

# =============================================================================
# Main
# =============================================================================

# Load thresholds if specified
if [[ -n "${THRESHOLD_FILE}" ]]; then
    load_thresholds "${THRESHOLD_FILE}"
fi

# Print header (skip if JSON output mode)
if [[ "${JSON_OUTPUT}" != "true" ]]; then
    echo ""
    echo -e "${BOLD}=== Benchmark Comparison ===${NC}"
    echo ""
    echo "Base: ${BASE_DIR}"
    echo "New:  ${NEW_DIR}"
fi

# Determine comparison mode
# Mode 1: Direct comparison (both directories have wrk.txt or wrk-output.json)
# Mode 2: Scenario comparison (directories have subdirectories)

if { [[ -f "${BASE_DIR}/wrk.txt" ]] || [[ -f "${BASE_DIR}/wrk-output.json" ]]; } && \
   { [[ -f "${NEW_DIR}/wrk.txt" ]] || [[ -f "${NEW_DIR}/wrk-output.json" ]]; }; then
    # Direct comparison
    scenario_name=$(basename "${BASE_DIR}")
    compare_single "${BASE_DIR}" "${NEW_DIR}" "${scenario_name}"
else
    # Find common scenarios
    base_scenarios=$(find_results "${BASE_DIR}")
    new_scenarios=$(find_results "${NEW_DIR}")

    # Find intersection and report missing scenarios
    compared=0
    missing_in_new=()
    missing_in_base=()

    for scenario in ${base_scenarios}; do
        base_path="${BASE_DIR}/${scenario}"
        new_path="${NEW_DIR}/${scenario}"

        if [[ -d "${new_path}" ]] || [[ "${scenario}" == "${BASE_DIR}" ]]; then
            if [[ "${scenario}" == "${BASE_DIR}" ]]; then
                compare_single "${BASE_DIR}" "${NEW_DIR}" "$(basename "${BASE_DIR}")"
            else
                compare_single "${base_path}" "${new_path}" "${scenario}"
            fi
            compared=$((compared + 1))
        else
            missing_in_new+=("${scenario}")
        fi
    done

    # Check for scenarios in new that are not in base
    for scenario in ${new_scenarios}; do
        check_base_path="${BASE_DIR}/${scenario}"
        if [[ ! -d "${check_base_path}" ]] && [[ "${scenario}" != "${NEW_DIR}" ]]; then
            missing_in_base+=("${scenario}")
        fi
    done

    # Report missing scenarios
    if [[ ${#missing_in_new[@]} -gt 0 ]]; then
        echo ""
        echo -e "${YELLOW}Scenarios in base but not in new:${NC}"
        for s in "${missing_in_new[@]}"; do
            echo "  - ${s}"
        done
    fi

    if [[ ${#missing_in_base[@]} -gt 0 ]]; then
        echo ""
        echo -e "${YELLOW}Scenarios in new but not in base:${NC}"
        for s in "${missing_in_base[@]}"; do
            echo "  - ${s}"
        done
    fi

    if [[ ${compared} -eq 0 ]]; then
        echo ""
        echo -e "${RED}No comparable results found between directories.${NC}"
        echo ""
        echo "Base scenarios: ${base_scenarios:-none}"
        echo "New scenarios:  ${new_scenarios:-none}"
        exit 2
    fi
fi

# =============================================================================
# Output Results
# =============================================================================

if [[ "${JSON_OUTPUT}" == "true" ]]; then
    # Build JSON array from collected results
    echo "{"
    echo "  \"base_dir\": \"${BASE_DIR}\","
    echo "  \"new_dir\": \"${NEW_DIR}\","
    echo "  \"regression_detected\": ${REGRESSION_DETECTED},"
    echo "  \"comparisons\": ["

    first=true
    for result in "${JSON_RESULTS[@]}"; do
        if [[ "${first}" == "true" ]]; then
            first=false
        else
            echo ","
        fi
        echo "${result}"
    done

    echo "  ]"
    echo "}"
else
    echo ""
    echo -e "${BOLD}=== Comparison Complete ===${NC}"
    echo ""
    echo -e "${GREEN}+${NC} = improvement (lower latency or higher throughput)"
    echo -e "${RED}-${NC} = regression (higher latency or lower throughput)"

    if [[ -n "${THRESHOLD_FILE}" ]]; then
        echo ""
        if [[ "${REGRESSION_DETECTED}" == "true" ]]; then
            echo -e "${RED}REGRESSION DETECTED: One or more metrics exceeded error thresholds.${NC}"
        else
            echo -e "${GREEN}All metrics within acceptable thresholds.${NC}"
        fi
    fi
    echo ""
fi

# Exit with appropriate code
if [[ "${REGRESSION_DETECTED}" == "true" ]]; then
    exit 3
fi
