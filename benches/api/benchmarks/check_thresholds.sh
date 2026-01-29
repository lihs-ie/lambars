#!/usr/bin/env bash
# benches/api/benchmarks/check_thresholds.sh
#
# Check performance thresholds for benchmark scenarios.
#
# This script validates that latency metrics (p50, p95, p99) and optional
# rate metrics (error_rate, conflict_rate) in meta.json meet the defined
# thresholds for each scenario.
#
# Dependencies:
#   - jq:  JSON parsing (required)
#   - bc:  Floating point comparison (required)
#   - yq:  YAML parsing for thresholds.yaml (required)
#
# Thresholds are defined in thresholds.yaml (single source of truth).
#
# Usage:
#   check_thresholds.sh <results_dir> <scenario>
#
# Arguments:
#   results_dir: Path to the benchmark results directory
#   scenario:    Name of the scenario (defined in thresholds.yaml)
#
# Exit codes:
#   0: Pass - All thresholds met
#   1: General error (invalid arguments, unknown scenario, missing dependencies)
#   2: Missing required metrics (meta.json not found or p50/p95/p99 missing)
#   3: Threshold exceeded (one or more metrics failed)
#
# Examples:
#   ./check_thresholds.sh ./results/20260124_120000 tasks_search_hot
#   ./check_thresholds.sh /path/to/results tasks_search_cold
#   ./check_thresholds.sh /path/to/results tasks_update_steady
#   ./check_thresholds.sh /path/to/results tasks_update_conflict

set -euo pipefail

# =============================================================================
# Configuration
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# =============================================================================
# Dependency Checks
# =============================================================================

# Check if jq is available
if ! command -v jq &> /dev/null; then
    echo "ERROR: jq is required but not installed"
    echo "Install with: brew install jq (macOS) or apt-get install jq (Linux)"
    exit 1
fi

# Check if bc is available
if ! command -v bc &> /dev/null; then
    echo "ERROR: bc is required but not installed"
    echo "Install with: brew install bc (macOS) or apt-get install bc (Linux)"
    exit 1
fi

# Check if yq is available
if ! command -v yq &> /dev/null; then
    echo "ERROR: yq is required but not installed"
    echo "Install with: brew install yq (macOS) or snap install yq (Linux)"
    exit 1
fi

# =============================================================================
# Threshold Functions
# =============================================================================

# Get threshold from thresholds.yaml (single source of truth)
# Arguments: scenario, metric (p50, p95, p99, error_rate, conflict_rate)
# Returns: threshold value or empty string
get_threshold() {
    local scenario="${1}"
    local metric="${2}"
    local yaml_file="${SCRIPT_DIR}/thresholds.yaml"

    if [[ ! -f "${yaml_file}" ]]; then
        echo ""
        return
    fi

    local yaml_key
    case "${metric}" in
        p50|p95|p99)
            yaml_key="${metric}_latency_ms"
            ;;
        error_rate|conflict_rate)
            yaml_key="${metric}"
            ;;
        *)
            echo ""
            return
            ;;
    esac

    local value
    value=$(yq ".scenarios.${scenario}.${yaml_key}.error // \"\"" "${yaml_file}" 2>/dev/null)

    if [[ -n "${value}" ]] && [[ "${value}" != "null" ]]; then
        echo "${value}"
    fi
}

# =============================================================================
# Argument Parsing
# =============================================================================

show_usage() {
    cat << 'EOF'
Usage: check_thresholds.sh <results_dir> <scenario>

Arguments:
  results_dir  Path to the benchmark results directory
  scenario     Name of the scenario (defined in thresholds.yaml)

Supported scenarios:
  Scenarios are defined in thresholds.yaml. Use `yq '.scenarios | keys' thresholds.yaml`
  to list available scenarios.

Exit codes:
  0: Pass - All thresholds met
  1: General error
  2: Missing required metrics
  3: Threshold exceeded
EOF
    exit 1
}

if [[ $# -lt 2 ]]; then
    echo "ERROR: Missing required arguments"
    show_usage
fi

RESULTS_DIR="${1}"
SCENARIO="${2}"

# =============================================================================
# Utility Functions
# =============================================================================

# Parse latency value and convert to milliseconds
# Handles formats: 1.23ms, 123.45us, 1.5s, or numeric values
parse_latency_to_ms() {
    local value="${1}"

    # Handle empty or null values
    if [[ -z "${value}" ]] || [[ "${value}" == "null" ]] || [[ "${value}" == "N/A" ]]; then
        echo ""
        return
    fi

    # Handle "0" as a special case (invalid data)
    if [[ "${value}" == "0" ]]; then
        echo ""
        return
    fi

    # Extract numeric part and unit using grep/sed for POSIX compatibility
    local numeric_value

    # Check for unit suffix
    if echo "${value}" | grep -qE '^[0-9.]+us$'; then
        numeric_value=$(echo "${value}" | sed 's/us$//')
        echo "scale=4; ${numeric_value} / 1000" | bc
    elif echo "${value}" | grep -qE '^[0-9.]+ms$'; then
        echo "${value}" | sed 's/ms$//'
    elif echo "${value}" | grep -qE '^[0-9.]+s$'; then
        numeric_value=$(echo "${value}" | sed 's/s$//')
        echo "scale=4; ${numeric_value} * 1000" | bc
    elif echo "${value}" | grep -qE '^[0-9.]+$'; then
        # Assume numeric value is already in milliseconds
        echo "${value}"
    else
        # Unknown format
        echo ""
    fi
}

# =============================================================================
# Main Logic
# =============================================================================

# Validate scenario by attempting to load thresholds
P50_MAX=$(get_threshold "${SCENARIO}" "p50")
P95_MAX=$(get_threshold "${SCENARIO}" "p95")
P99_MAX=$(get_threshold "${SCENARIO}" "p99")
ERROR_RATE_MAX=$(get_threshold "${SCENARIO}" "error_rate")
CONFLICT_RATE_MAX=$(get_threshold "${SCENARIO}" "conflict_rate")

if [[ -z "${P50_MAX}" ]]; then
    echo "ERROR: Unknown scenario: ${SCENARIO}"
    echo "Scenarios are defined in thresholds.yaml"
    echo "Use: yq '.scenarios | keys' ${SCRIPT_DIR}/thresholds.yaml"
    exit 1
fi

# Locate meta.json file
# Try multiple possible paths:
#   1. <results_dir>/<scenario>/benchmark/meta/<scenario>.json
#   2. <results_dir>/<scenario>/meta.json
#   3. <results_dir>/benchmark/meta/<scenario>.json
#   4. <results_dir>/meta.json
META_FILE=""
declare -a POSSIBLE_PATHS=(
    "${RESULTS_DIR}/${SCENARIO}/benchmark/meta/${SCENARIO}.json"
    "${RESULTS_DIR}/${SCENARIO}/meta.json"
    "${RESULTS_DIR}/benchmark/meta/${SCENARIO}.json"
    "${RESULTS_DIR}/meta.json"
)

for path in "${POSSIBLE_PATHS[@]}"; do
    if [[ -f "${path}" ]]; then
        META_FILE="${path}"
        break
    fi
done

if [[ -z "${META_FILE}" ]]; then
    echo "ERROR: Meta file not found for scenario '${SCENARIO}'"
    echo "Searched paths:"
    for path in "${POSSIBLE_PATHS[@]}"; do
        echo "  - ${path}"
    done
    exit 2
fi

echo "Checking thresholds for scenario: ${SCENARIO}"
echo "Meta file: ${META_FILE}"

# Extract latency values from meta.json
# Values are under .results.p50, .results.p95, .results.p99
RAW_P50=$(jq -r '.results.p50 // empty' "${META_FILE}" 2>/dev/null || true)
RAW_P95=$(jq -r '.results.p95 // empty' "${META_FILE}" 2>/dev/null || true)
RAW_P99=$(jq -r '.results.p99 // empty' "${META_FILE}" 2>/dev/null || true)

# Extract rate values from meta.json (optional)
# Values are under .results.error_rate, .results.conflict_rate
RAW_ERROR_RATE=$(jq -r '.results.error_rate // empty' "${META_FILE}" 2>/dev/null || true)
RAW_CONFLICT_RATE=$(jq -r '.results.conflict_rate // empty' "${META_FILE}" 2>/dev/null || true)

# Convert to milliseconds
P50=$(parse_latency_to_ms "${RAW_P50}")
P95=$(parse_latency_to_ms "${RAW_P95}")
P99=$(parse_latency_to_ms "${RAW_P99}")

# Rate values are already decimal (no conversion needed)
ERROR_RATE="${RAW_ERROR_RATE}"
CONFLICT_RATE="${RAW_CONFLICT_RATE}"

# Validate that required metrics exist
MISSING_METRICS=""

if [[ -z "${P50}" ]]; then
    MISSING_METRICS="${MISSING_METRICS} p50"
fi
if [[ -z "${P95}" ]]; then
    MISSING_METRICS="${MISSING_METRICS} p95"
fi
if [[ -z "${P99}" ]]; then
    MISSING_METRICS="${MISSING_METRICS} p99"
fi

# Validate rate metrics for scenarios that require them
if [[ -n "${ERROR_RATE_MAX}" ]] && [[ -z "${ERROR_RATE}" ]]; then
    MISSING_METRICS="${MISSING_METRICS} error_rate"
fi
if [[ -n "${CONFLICT_RATE_MAX}" ]] && [[ -z "${CONFLICT_RATE}" ]]; then
    MISSING_METRICS="${MISSING_METRICS} conflict_rate"
fi

if [[ -n "${MISSING_METRICS}" ]]; then
    echo "ERROR: Missing required metrics in meta.json:${MISSING_METRICS}"
    echo "Raw values: p50='${RAW_P50}', p95='${RAW_P95}', p99='${RAW_P99}'"
    if [[ -n "${ERROR_RATE_MAX}" ]]; then
        echo "  error_rate='${RAW_ERROR_RATE}' (required)"
    fi
    if [[ -n "${CONFLICT_RATE_MAX}" ]]; then
        echo "  conflict_rate='${RAW_CONFLICT_RATE}' (required)"
    fi
    exit 2
fi

echo ""
echo "Thresholds:"
echo "  p50 <= ${P50_MAX}ms"
echo "  p95 <= ${P95_MAX}ms"
echo "  p99 <= ${P99_MAX}ms"
if [[ -n "${ERROR_RATE_MAX}" ]]; then
    echo "  error_rate <= ${ERROR_RATE_MAX}"
fi
if [[ -n "${CONFLICT_RATE_MAX}" ]]; then
    echo "  conflict_rate <= ${CONFLICT_RATE_MAX}"
fi
echo ""
echo "Results:"
echo "  p50 = ${P50}ms"
echo "  p95 = ${P95}ms"
echo "  p99 = ${P99}ms"
if [[ -n "${ERROR_RATE}" ]]; then
    echo "  error_rate = ${ERROR_RATE}"
fi
if [[ -n "${CONFLICT_RATE}" ]]; then
    echo "  conflict_rate = ${CONFLICT_RATE}"
fi
echo ""

# Check thresholds
FAILED=0
FAILURES=""

if (( $(echo "${P50} > ${P50_MAX}" | bc -l) )); then
    FAILURES="${FAILURES}
  - p50=${P50}ms exceeds threshold of ${P50_MAX}ms"
    FAILED=1
fi

if (( $(echo "${P95} > ${P95_MAX}" | bc -l) )); then
    FAILURES="${FAILURES}
  - p95=${P95}ms exceeds threshold of ${P95_MAX}ms"
    FAILED=1
fi

if (( $(echo "${P99} > ${P99_MAX}" | bc -l) )); then
    FAILURES="${FAILURES}
  - p99=${P99}ms exceeds threshold of ${P99_MAX}ms"
    FAILED=1
fi

# Check error_rate threshold (required for scenarios with threshold)
if [[ -n "${ERROR_RATE_MAX}" ]] && [[ -n "${ERROR_RATE}" ]]; then
    if (( $(echo "${ERROR_RATE} >= ${ERROR_RATE_MAX}" | bc -l) )); then
        FAILURES="${FAILURES}
  - error_rate=${ERROR_RATE} exceeds or equals threshold of ${ERROR_RATE_MAX}"
        FAILED=1
    fi
fi

# Check conflict_rate threshold (required for scenarios with threshold)
if [[ -n "${CONFLICT_RATE_MAX}" ]] && [[ -n "${CONFLICT_RATE}" ]]; then
    if (( $(echo "${CONFLICT_RATE} >= ${CONFLICT_RATE_MAX}" | bc -l) )); then
        FAILURES="${FAILURES}
  - conflict_rate=${CONFLICT_RATE} exceeds or equals threshold of ${CONFLICT_RATE_MAX}"
        FAILED=1
    fi
fi

# Build summary strings
RESULTS_SUMMARY="p50=${P50}ms, p95=${P95}ms, p99=${P99}ms"
THRESHOLDS_SUMMARY="p50<=${P50_MAX}ms, p95<=${P95_MAX}ms, p99<=${P99_MAX}ms"

if [[ -n "${ERROR_RATE}" ]]; then
    RESULTS_SUMMARY="${RESULTS_SUMMARY}, error_rate=${ERROR_RATE}"
fi
if [[ -n "${ERROR_RATE_MAX}" ]]; then
    THRESHOLDS_SUMMARY="${THRESHOLDS_SUMMARY}, error_rate<=${ERROR_RATE_MAX}"
fi
if [[ -n "${CONFLICT_RATE}" ]]; then
    RESULTS_SUMMARY="${RESULTS_SUMMARY}, conflict_rate=${CONFLICT_RATE}"
fi
if [[ -n "${CONFLICT_RATE_MAX}" ]]; then
    THRESHOLDS_SUMMARY="${THRESHOLDS_SUMMARY}, conflict_rate<=${CONFLICT_RATE_MAX}"
fi

# Output result
if [[ ${FAILED} -eq 1 ]]; then
    echo "FAIL: Threshold(s) exceeded${FAILURES}"
    echo ""
    echo "---"
    echo "Summary:"
    echo "  Scenario: ${SCENARIO}"
    echo "  Results: ${RESULTS_SUMMARY}"
    echo "  Thresholds: ${THRESHOLDS_SUMMARY}"
    exit 3
fi

echo "PASS: All thresholds met"
echo ""
echo "Summary:"
echo "  Scenario: ${SCENARIO}"
echo "  Results: ${RESULTS_SUMMARY}"
echo "  Thresholds: ${THRESHOLDS_SUMMARY}"
exit 0
