#!/usr/bin/env bash
# benches/api/benchmarks/check_thresholds.sh
#
# Check performance thresholds for search scenarios (REQ-SEARCH-MET-001).
#
# This script validates that latency metrics (p50, p95, p99) in meta.json
# meet the defined thresholds for search scenarios.
#
# Usage:
#   check_thresholds.sh <results_dir> <scenario>
#
# Arguments:
#   results_dir: Path to the benchmark results directory
#   scenario:    Name of the scenario (e.g., tasks_search_hot, tasks_search_cold)
#
# Exit codes:
#   0: Pass - All thresholds met
#   1: General error (invalid arguments, unknown scenario)
#   2: Missing required metrics (meta.json not found or p50/p95/p99 missing)
#   3: Threshold exceeded (one or more metrics failed)
#
# Examples:
#   ./check_thresholds.sh ./results/20260124_120000 tasks_search_hot
#   ./check_thresholds.sh /path/to/results tasks_search_cold

set -euo pipefail

# =============================================================================
# Configuration
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Get thresholds for a scenario
# Arguments: scenario_name, metric_type (p50, p95, p99)
# Returns: threshold value in ms
get_threshold() {
    local scenario="${1}"
    local metric="${2}"

    case "${scenario}" in
        tasks_search_hot)
            case "${metric}" in
                p50) echo "50" ;;
                p95) echo "200" ;;
                p99) echo "400" ;;
            esac
            ;;
        tasks_search_cold)
            case "${metric}" in
                p50) echo "100" ;;
                p95) echo "300" ;;
                p99) echo "500" ;;
            esac
            ;;
        *)
            echo ""
            ;;
    esac
}

# =============================================================================
# Argument Parsing
# =============================================================================

show_usage() {
    cat << 'EOF'
Usage: check_thresholds.sh <results_dir> <scenario>

Arguments:
  results_dir  Path to the benchmark results directory
  scenario     Name of the scenario (e.g., tasks_search_hot, tasks_search_cold)

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
    local unit

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

# Load thresholds from thresholds.yaml if yq is available
# Arguments: scenario, metric (p50, p95, p99)
# Returns: threshold value or empty string
load_threshold_from_yaml() {
    local scenario="${1}"
    local metric="${2}"
    local yaml_file="${SCRIPT_DIR}/thresholds.yaml"

    if [[ ! -f "${yaml_file}" ]]; then
        return
    fi

    if command -v yq &> /dev/null; then
        local yaml_key="${metric}_latency_ms"
        local value
        value=$(yq ".scenarios.${scenario}.${yaml_key}.error // \"\"" "${yaml_file}" 2>/dev/null)

        if [[ -n "${value}" ]] && [[ "${value}" != "null" ]]; then
            echo "${value}"
        fi
    fi
}

# =============================================================================
# Main Logic
# =============================================================================

# Validate scenario
P50_MAX=$(get_threshold "${SCENARIO}" "p50")
P95_MAX=$(get_threshold "${SCENARIO}" "p95")
P99_MAX=$(get_threshold "${SCENARIO}" "p99")

if [[ -z "${P50_MAX}" ]]; then
    echo "ERROR: Unknown scenario: ${SCENARIO}"
    echo "Supported scenarios: tasks_search_hot, tasks_search_cold"
    exit 1
fi

# Try to load thresholds from YAML (may override defaults)
YAML_P50=$(load_threshold_from_yaml "${SCENARIO}" "p50")
YAML_P95=$(load_threshold_from_yaml "${SCENARIO}" "p95")
YAML_P99=$(load_threshold_from_yaml "${SCENARIO}" "p99")

if [[ -n "${YAML_P50}" ]]; then P50_MAX="${YAML_P50}"; fi
if [[ -n "${YAML_P95}" ]]; then P95_MAX="${YAML_P95}"; fi
if [[ -n "${YAML_P99}" ]]; then P99_MAX="${YAML_P99}"; fi

# Locate meta.json file
# Try multiple possible paths:
#   1. <results_dir>/<scenario>/benchmark/meta/<scenario>.json
#   2. <results_dir>/<scenario>/meta.json
#   3. <results_dir>/benchmark/meta/<scenario>.json
#   4. <results_dir>/meta.json
META_FILE=""
POSSIBLE_PATHS="${RESULTS_DIR}/${SCENARIO}/benchmark/meta/${SCENARIO}.json
${RESULTS_DIR}/${SCENARIO}/meta.json
${RESULTS_DIR}/benchmark/meta/${SCENARIO}.json
${RESULTS_DIR}/meta.json"

for path in ${POSSIBLE_PATHS}; do
    if [[ -f "${path}" ]]; then
        META_FILE="${path}"
        break
    fi
done

if [[ -z "${META_FILE}" ]]; then
    echo "ERROR: Meta file not found for scenario '${SCENARIO}'"
    echo "Searched paths:"
    for path in ${POSSIBLE_PATHS}; do
        echo "  - ${path}"
    done
    exit 2
fi

echo "Checking thresholds for scenario: ${SCENARIO}"
echo "Meta file: ${META_FILE}"

# Check if jq is available
if ! command -v jq &> /dev/null; then
    echo "ERROR: jq is required but not installed"
    exit 1
fi

# Check if bc is available
if ! command -v bc &> /dev/null; then
    echo "ERROR: bc is required but not installed"
    exit 1
fi

# Extract latency values from meta.json
# Values are under .results.p50, .results.p95, .results.p99
RAW_P50=$(jq -r '.results.p50 // empty' "${META_FILE}" 2>/dev/null || true)
RAW_P95=$(jq -r '.results.p95 // empty' "${META_FILE}" 2>/dev/null || true)
RAW_P99=$(jq -r '.results.p99 // empty' "${META_FILE}" 2>/dev/null || true)

# Convert to milliseconds
P50=$(parse_latency_to_ms "${RAW_P50}")
P95=$(parse_latency_to_ms "${RAW_P95}")
P99=$(parse_latency_to_ms "${RAW_P99}")

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

if [[ -n "${MISSING_METRICS}" ]]; then
    echo "ERROR: Missing required metrics in meta.json:${MISSING_METRICS}"
    echo "Raw values: p50='${RAW_P50}', p95='${RAW_P95}', p99='${RAW_P99}'"
    exit 2
fi

echo ""
echo "Thresholds:"
echo "  p50 <= ${P50_MAX}ms"
echo "  p95 <= ${P95_MAX}ms"
echo "  p99 <= ${P99_MAX}ms"
echo ""
echo "Results:"
echo "  p50 = ${P50}ms"
echo "  p95 = ${P95}ms"
echo "  p99 = ${P99}ms"
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

# Output result
if [[ ${FAILED} -eq 1 ]]; then
    echo "FAIL: Threshold(s) exceeded${FAILURES}"
    echo ""
    echo "---"
    echo "Summary:"
    echo "  Scenario: ${SCENARIO}"
    echo "  Results: p50=${P50}ms, p95=${P95}ms, p99=${P99}ms"
    echo "  Thresholds: p50<=${P50_MAX}ms, p95<=${P95_MAX}ms, p99<=${P99_MAX}ms"
    exit 3
fi

echo "PASS: All thresholds met"
echo ""
echo "Summary:"
echo "  Scenario: ${SCENARIO}"
echo "  Results: p50=${P50}ms, p95=${P95}ms, p99=${P99}ms"
echo "  Thresholds: p50<=${P50_MAX}ms, p95<=${P95_MAX}ms, p99<=${P99_MAX}ms"
exit 0
