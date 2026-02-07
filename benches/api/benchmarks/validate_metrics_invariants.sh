#!/usr/bin/env bash
set -euo pipefail

# validate_metrics_invariants.sh
# Validates metrics invariants across all benchmark scenarios
#
# Usage: validate_metrics_invariants.sh [--all <directory>] [--report <path>] [<meta.json>...]
# Exit code: 0 = all pass, 1 = violations found

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
ALL_MODE=false
ALL_DIR=""
REPORT_FILE=""
declare -a META_FILES=()

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --all)
            ALL_MODE=true
            ALL_DIR="$2"
            shift 2
            ;;
        --report)
            REPORT_FILE="$2"
            shift 2
            ;;
        *)
            META_FILES+=("$1")
            shift
            ;;
    esac
done

# If --all mode, find all meta.json files recursively
if [[ "${ALL_MODE}" == "true" ]]; then
    if [[ -z "${ALL_DIR}" || ! -d "${ALL_DIR}" ]]; then
        echo -e "${RED}ERROR: --all requires a valid directory${NC}" >&2
        exit 1
    fi
    echo "Searching for meta.json files in ${ALL_DIR}..." >&2
    while IFS= read -r -d '' file; do
        META_FILES+=("$file")
    done < <(
        find "${ALL_DIR}" -type f \
            \( -name 'meta.json' -o -path '*/benchmark/meta/*.json' \) \
            -print0 2>/dev/null
    )

    if [[ ${#META_FILES[@]} -eq 0 ]]; then
        echo -e "${RED}ERROR: No metrics JSON files found in ${ALL_DIR}${NC}" >&2
        exit 1
    fi

    echo "Found ${#META_FILES[@]} meta.json files" >&2
fi

# Check if jq is available
if ! command -v jq &> /dev/null; then
    echo -e "${RED}ERROR: jq is required but not installed${NC}" >&2
    exit 1
fi

# Validation results
declare -a VIOLATIONS=()
PASS_COUNT=0
FAIL_COUNT=0

# Report buffer
REPORT=""

# Add to report
add_report() {
    REPORT+="$1"$'\n'
}

# Validate a single meta.json file
validate_meta_json() {
    local meta_file="$1"
    local file_name
    file_name=$(basename "$(dirname "${meta_file}")")

    # Skip if file doesn't exist or is not readable
    if [[ ! -f "${meta_file}" || ! -r "${meta_file}" ]]; then
        echo -e "${YELLOW}SKIP: ${file_name} (file not found or not readable)${NC}" >&2
        add_report "SKIP: ${file_name} (file not found or not readable)"
        return
    fi

    # Check if file is valid JSON
    if ! jq -e . "${meta_file}" >/dev/null 2>&1; then
        echo -e "${RED}FAIL: ${file_name} (invalid JSON)${NC}" >&2
        add_report "FAIL: ${file_name} (invalid JSON)"
        VIOLATIONS+=("${file_name}: invalid JSON")
        FAIL_COUNT=$((FAIL_COUNT + 1))
        return
    fi

    local violations_for_file=()

    # Extract metrics
    local requests status_sum http_status error_rate http_4xx http_5xx socket_errors
    local p50 p95 p99

    requests=$(jq -r '.results.requests // 0' "${meta_file}" 2>/dev/null)
    status_sum=$(jq '[.results.http_status | to_entries[] | .value] | add // 0' "${meta_file}" 2>/dev/null)
    error_rate=$(jq -r '.results.error_rate // 0' "${meta_file}" 2>/dev/null)
    http_4xx=$(jq -r '.errors.http_4xx // 0' "${meta_file}" 2>/dev/null)
    http_5xx=$(jq -r '.errors.http_5xx // 0' "${meta_file}" 2>/dev/null)
    socket_errors=$(jq -r '.errors.socket_errors.total // 0' "${meta_file}" 2>/dev/null)
    p50=$(jq -r '.results.latency_ms.p50 // null' "${meta_file}" 2>/dev/null)
    p95=$(jq -r '.results.latency_ms.p95 // null' "${meta_file}" 2>/dev/null)
    p99=$(jq -r '.results.latency_ms.p99 // null' "${meta_file}" 2>/dev/null)

    # Invariant 1: Status coverage (REQ-MET-P3-001)
    # sum(http_status.*) == requests
    if (( 10#${requests} > 0 )); then
        if (( 10#${status_sum} != 10#${requests} )); then
            violations_for_file+=("Status coverage: ${status_sum}/${requests} != 1.0000")
        fi
    fi

    # Invariant 2: Error rate consistency (REQ-MET-P3-002)
    # |error_rate - (http_4xx + http_5xx + socket_errors) / requests| <= 1e-9
    if (( 10#${requests} > 0 )); then
        local recomputed
        recomputed=$(awk -v h4="${http_4xx}" -v h5="${http_5xx}" -v se="${socket_errors}" -v req="${requests}" 'BEGIN {
            total_errors = h4 + h5 + se
            rate = total_errors / req
            printf "%.12f", rate
        }')
        local diff
        diff=$(awk -v er="${error_rate}" -v rc="${recomputed}" 'BEGIN {
            d = er - rc
            if (d < 0) d = -d
            printf "%.12f", d
        }')
        local threshold="0.000001"
        if awk -v d="${diff}" -v t="${threshold}" 'BEGIN { exit !(d > t) }'; then
            violations_for_file+=("Error rate inconsistency: error_rate=${error_rate}, recomputed=${recomputed}, diff=${diff}")
        fi
    fi

    # Invariant 3: Latency completeness (REQ-MET-P3-003)
    # requests > 0 => p50, p99 are non-null
    # Note: p95 is not checked because wrk2 --latency does not output 95th percentile
    if (( 10#${requests} > 0 )); then
        local missing_percentiles=()
        [[ "${p50}" == "null" ]] && missing_percentiles+=("p50")
        [[ "${p99}" == "null" ]] && missing_percentiles+=("p99")

        if [[ ${#missing_percentiles[@]} -gt 0 ]]; then
            violations_for_file+=("Missing percentiles: ${missing_percentiles[*]}")
        fi
    fi

    # Report results
    if [[ ${#violations_for_file[@]} -eq 0 ]]; then
        echo -e "${GREEN}PASS: ${file_name}${NC}" >&2
        add_report "PASS: ${file_name}"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo -e "${RED}FAIL: ${file_name}${NC}" >&2
        add_report "FAIL: ${file_name}"
        for violation in "${violations_for_file[@]}"; do
            echo -e "  ${YELLOW}> ${violation}${NC}" >&2
            add_report "  > ${violation}"
            VIOLATIONS+=("${file_name}: ${violation}")
        done
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
}

# Main validation loop
echo "Validating ${#META_FILES[@]} meta.json files..." >&2
add_report "=== Metrics Invariant Validation Report ==="
add_report "Date: $(date)"
add_report "Files: ${#META_FILES[@]}"
add_report ""

for meta_file in "${META_FILES[@]}"; do
    validate_meta_json "${meta_file}"
done

# Summary
add_report ""
add_report "=== Summary ==="
add_report "Total: ${#META_FILES[@]}"
add_report "Pass: ${PASS_COUNT}"
add_report "Fail: ${FAIL_COUNT}"

# Write report if specified
if [[ -n "${REPORT_FILE}" ]]; then
    mkdir -p "$(dirname "${REPORT_FILE}")"
    echo -e "${REPORT}" > "${REPORT_FILE}"
    echo "Report written to ${REPORT_FILE}" >&2
fi

# Exit code
if [[ ${#VIOLATIONS[@]} -gt 0 ]]; then
    echo "" >&2
    echo -e "${RED}=== Violations Summary ===${NC}" >&2
    for violation in "${VIOLATIONS[@]}"; do
        echo -e "${RED}  ${violation}${NC}" >&2
    done
    echo "" >&2
    echo -e "${RED}Total violations: ${#VIOLATIONS[@]}${NC}" >&2
    exit 1
else
    echo "" >&2
    echo -e "${GREEN}All metrics invariants passed (${PASS_COUNT}/${#META_FILES[@]})${NC}" >&2
    exit 0
fi
