#!/bin/bash
# benches/api/benchmarks/compare_results.sh
#
# Compare two benchmark result directories and report differences.
#
# Usage:
#   ./compare_results.sh <base_dir> <new_dir>
#   ./compare_results.sh results/20260122_120000 results/20260123_140000
#   ./compare_results.sh results/20260122_120000/tasks_search results/20260123_140000/tasks_search
#
# Output:
#   - Comparison table with metrics: p50, p95, p99, rps, error_rate
#   - Percentage differences (positive = improvement, negative = regression)
#   - Memory metrics if profile data available
#
# Exit codes:
#   0 - Success
#   1 - Missing arguments or directories
#   2 - No comparable results found

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

if [[ $# -lt 2 ]]; then
    echo "Usage: $0 <base_dir> <new_dir>"
    echo ""
    echo "Examples:"
    echo "  $0 results/20260122_120000 results/20260123_140000"
    echo "  $0 results/baseline/tasks_search results/optimized/tasks_search"
    exit 1
fi

BASE_DIR="$1"
NEW_DIR="$2"

if [[ ! -d "${BASE_DIR}" ]]; then
    echo -e "${RED}Error: Base directory not found: ${BASE_DIR}${NC}"
    exit 1
fi

if [[ ! -d "${NEW_DIR}" ]]; then
    echo -e "${RED}Error: New directory not found: ${NEW_DIR}${NC}"
    exit 1
fi

# =============================================================================
# Utility Functions
# =============================================================================

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

# =============================================================================
# Result Collection
# =============================================================================

# Find all result files (wrk.txt or meta.json) in a directory
find_results() {
    local dir="$1"
    local results=()

    # Check if directory has direct wrk.txt (single scenario)
    if [[ -f "${dir}/wrk.txt" ]]; then
        echo "${dir}"
        return
    fi

    # Check subdirectories
    for subdir in "${dir}"/*/; do
        if [[ -d "${subdir}" ]]; then
            if [[ -f "${subdir}/wrk.txt" ]] || [[ -f "${subdir}/meta.json" ]]; then
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
    local base_meta="${base_path}/meta.json"
    local new_meta="${new_path}/meta.json"

    # Extract metrics from wrk.txt
    local base_rps new_rps
    local base_p50 new_p50
    local base_p95 new_p95
    local base_p99 new_p99
    local base_avg new_avg

    base_rps=$(extract_from_wrk "${base_wrk}" "Requests/sec:")
    new_rps=$(extract_from_wrk "${new_wrk}" "Requests/sec:")

    base_p50=$(extract_from_wrk "${base_wrk}" "50%")
    new_p50=$(extract_from_wrk "${new_wrk}" "50%")

    base_p95=$(extract_from_wrk "${base_wrk}" "95%")
    new_p95=$(extract_from_wrk "${new_wrk}" "95%")

    base_p99=$(extract_from_wrk "${base_wrk}" "99%")
    new_p99=$(extract_from_wrk "${new_wrk}" "99%")

    base_avg=$(extract_from_wrk "${base_wrk}" "Latency")
    new_avg=$(extract_from_wrk "${new_wrk}" "Latency")

    # Extract error rate from meta.json if available
    local base_error new_error
    base_error=$(extract_from_meta "${base_meta}" "error_rate")
    new_error=$(extract_from_meta "${new_meta}" "error_rate")

    # Convert latencies to ms for comparison
    local base_p50_ms new_p50_ms
    local base_p95_ms new_p95_ms
    local base_p99_ms new_p99_ms

    base_p50_ms=$(parse_latency_ms "${base_p50}")
    new_p50_ms=$(parse_latency_ms "${new_p50}")

    base_p95_ms=$(parse_latency_ms "${base_p95}")
    new_p95_ms=$(parse_latency_ms "${new_p95}")

    base_p99_ms=$(parse_latency_ms "${base_p99}")
    new_p99_ms=$(parse_latency_ms "${new_p99}")

    # Calculate differences
    local diff_rps diff_p50 diff_p95 diff_p99 diff_error

    diff_rps=$(calc_diff_percent "${base_rps}" "${new_rps}" "throughput")
    diff_p50=$(calc_diff_percent "${base_p50_ms}" "${new_p50_ms}" "latency")
    diff_p95=$(calc_diff_percent "${base_p95_ms}" "${new_p95_ms}" "latency")
    diff_p99=$(calc_diff_percent "${base_p99_ms}" "${new_p99_ms}" "latency")

    if [[ "${base_error}" != "N/A" ]] && [[ "${new_error}" != "N/A" ]]; then
        diff_error=$(calc_diff_percent "${base_error}" "${new_error}" "latency")
    else
        diff_error="N/A"
    fi

    # Print comparison table
    echo ""
    echo -e "${BOLD}${CYAN}=== ${scenario_name} ===${NC}"
    echo ""
    printf "%-12s | %-12s | %-12s | %-10s\n" "Metric" "Base" "New" "Diff"
    printf "%-12s-+-%-12s-+-%-12s-+-%-10s\n" "------------" "------------" "------------" "----------"
    printf "%-12s | %-12s | %-12s | %s\n" "p50" "${base_p50:-N/A}" "${new_p50:-N/A}" "$(format_diff "${diff_p50}" "latency")"
    printf "%-12s | %-12s | %-12s | %s\n" "p95" "${base_p95:-N/A}" "${new_p95:-N/A}" "$(format_diff "${diff_p95}" "latency")"
    printf "%-12s | %-12s | %-12s | %s\n" "p99" "${base_p99:-N/A}" "${new_p99:-N/A}" "$(format_diff "${diff_p99}" "latency")"
    printf "%-12s | %-12s | %-12s | %s\n" "rps" "${base_rps:-N/A}" "${new_rps:-N/A}" "$(format_diff "${diff_rps}" "throughput")"

    if [[ "${base_error}" != "N/A" ]] || [[ "${new_error}" != "N/A" ]]; then
        printf "%-12s | %-12s | %-12s | %s\n" "error_rate" "${base_error:-N/A}" "${new_error:-N/A}" "$(format_diff "${diff_error}" "latency")"
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
}

# =============================================================================
# Main
# =============================================================================

echo ""
echo -e "${BOLD}=== Benchmark Comparison ===${NC}"
echo ""
echo "Base: ${BASE_DIR}"
echo "New:  ${NEW_DIR}"

# Determine comparison mode
# Mode 1: Direct comparison (both directories have wrk.txt)
# Mode 2: Scenario comparison (directories have subdirectories)

if [[ -f "${BASE_DIR}/wrk.txt" ]] && [[ -f "${NEW_DIR}/wrk.txt" ]]; then
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
        local check_base_path="${BASE_DIR}/${scenario}"
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

echo ""
echo -e "${BOLD}=== Comparison Complete ===${NC}"
echo ""
echo -e "${GREEN}+${NC} = improvement (lower latency or higher throughput)"
echo -e "${RED}-${NC} = regression (higher latency or lower throughput)"
echo ""
