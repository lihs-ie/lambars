#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

for cmd in jq bc yq; do
    if ! command -v "$cmd" &> /dev/null; then
        echo "ERROR: $cmd is required but not installed"
        exit 1
    fi
done

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
        p50|p90|p99)
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

show_usage() {
    cat << 'EOF'
Usage: check_thresholds.sh <results_dir> <scenario>

Arguments:
  results_dir  Path to benchmark results directory
  scenario     Scenario name (defined in thresholds.yaml)

Exit codes: 0=pass, 1=error, 2=missing metrics, 3=threshold exceeded
EOF
    exit 1
}

if [[ $# -lt 2 ]]; then
    echo "ERROR: Missing required arguments"
    show_usage
fi

RESULTS_DIR="${1}"
SCENARIO="${2}"

parse_latency_to_ms() {
    local value="${1}"

    if [[ -z "${value}" ]] || [[ "${value}" == "null" ]] || [[ "${value}" == "N/A" ]] || [[ "${value}" == "0" ]]; then
        echo ""
        return
    fi

    local numeric_value
    if echo "${value}" | grep -qE '^[0-9.]+us$'; then
        numeric_value=$(echo "${value}" | sed 's/us$//')
        echo "scale=4; ${numeric_value} / 1000" | bc
    elif echo "${value}" | grep -qE '^[0-9.]+ms$'; then
        echo "${value}" | sed 's/ms$//'
    elif echo "${value}" | grep -qE '^[0-9.]+s$'; then
        numeric_value=$(echo "${value}" | sed 's/s$//')
        echo "scale=4; ${numeric_value} * 1000" | bc
    elif echo "${value}" | grep -qE '^[0-9.]+$'; then
        echo "${value}"
    else
        echo ""
    fi
}

# Validate scenario
P50_MAX=$(get_threshold "${SCENARIO}" "p50")
P90_MAX=$(get_threshold "${SCENARIO}" "p90")
P99_MAX=$(get_threshold "${SCENARIO}" "p99")
ERROR_RATE_MAX=$(get_threshold "${SCENARIO}" "error_rate")
CONFLICT_RATE_MAX=$(get_threshold "${SCENARIO}" "conflict_rate")

if [[ -z "${P50_MAX}" ]]; then
    echo "ERROR: Unknown scenario: ${SCENARIO}"
    echo "Scenarios are defined in thresholds.yaml"
    echo "Use: yq '.scenarios | keys' ${SCRIPT_DIR}/thresholds.yaml"
    exit 1
fi

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

RAW_P50=$(jq -r '.results.p50 // .results.latency_ms.p50 // empty' "${META_FILE}" 2>/dev/null || true)
RAW_P90=$(jq -r '.results.p90 // .results.latency_ms.p90 // empty' "${META_FILE}" 2>/dev/null || true)
RAW_P99=$(jq -r '.results.p99 // .results.latency_ms.p99 // empty' "${META_FILE}" 2>/dev/null || true)
RAW_ERROR_RATE=$(jq -r '.results.error_rate // empty' "${META_FILE}" 2>/dev/null || true)
RAW_CONFLICT_RATE=$(jq -r '.results.conflict_rate // empty' "${META_FILE}" 2>/dev/null || true)

P50=$(parse_latency_to_ms "${RAW_P50}")
P90=$(parse_latency_to_ms "${RAW_P90}")
P99=$(parse_latency_to_ms "${RAW_P99}")
ERROR_RATE="${RAW_ERROR_RATE}"
CONFLICT_RATE="${RAW_CONFLICT_RATE}"

MISSING_METRICS=""
[[ -z "${P50}" ]] && MISSING_METRICS="${MISSING_METRICS} p50"
[[ -z "${P90}" ]] && MISSING_METRICS="${MISSING_METRICS} p90"
[[ -z "${P99}" ]] && MISSING_METRICS="${MISSING_METRICS} p99"
[[ -n "${ERROR_RATE_MAX}" ]] && [[ -z "${ERROR_RATE}" ]] && MISSING_METRICS="${MISSING_METRICS} error_rate"
[[ -n "${CONFLICT_RATE_MAX}" ]] && [[ -z "${CONFLICT_RATE}" ]] && MISSING_METRICS="${MISSING_METRICS} conflict_rate"

if [[ -n "${MISSING_METRICS}" ]]; then
    echo "ERROR: Missing required metrics in meta.json:${MISSING_METRICS}"
    echo "Raw values: p50='${RAW_P50}', p90='${RAW_P90}', p99='${RAW_P99}'"
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
echo "  p90 <= ${P90_MAX}ms"
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
echo "  p90 = ${P90}ms"
echo "  p99 = ${P99}ms"
if [[ -n "${ERROR_RATE}" ]]; then
    echo "  error_rate = ${ERROR_RATE}"
fi
if [[ -n "${CONFLICT_RATE}" ]]; then
    echo "  conflict_rate = ${CONFLICT_RATE}"
fi

declare -A CONFLICT_DETAIL_FIELDS=(
    [stale_version]="stale_version"
    [retryable_cas]="retryable_cas"
    [retry_success]="retry_success"
    [retry_exhausted]="retry_exhausted"
)
CONFLICT_DETAIL_VALUES=()
for field in "${!CONFLICT_DETAIL_FIELDS[@]}"; do
    value=$(jq -r "(.results.conflict_detail // .conflict_detail // {}).${field} // empty" "${META_FILE}" 2>/dev/null || true)
    [[ -n "${value}" ]] && CONFLICT_DETAIL_VALUES+=("${field}=${value}")
done

if [[ ${#CONFLICT_DETAIL_VALUES[@]} -gt 0 ]]; then
    echo "  conflict_detail:"
    for entry in "${CONFLICT_DETAIL_VALUES[@]}"; do
        echo "    ${entry}"
    done
fi

echo ""

check_validation_gate() {
    local fail_message="$1"
    local STATUS_400
    local STATUS_409
    local REQUESTS

    STATUS_400=$(jq -r '.results.http_status."400" // 0' "${META_FILE}")
    STATUS_409=$(jq -r '.results.http_status."409" // 0' "${META_FILE}")

    if [[ ! "${STATUS_400}" =~ ^[0-9]+$ || ! "${STATUS_409}" =~ ^[0-9]+$ ]]; then
        echo "ERROR: results.http_status.{400,409} must be non-negative integers (got: 400=${STATUS_400}, 409=${STATUS_409})"
        exit 2
    fi

    if (( 10#${STATUS_400} > 0 )); then
        echo "FAIL: ${fail_message}"
        echo "  http_status.400 = ${STATUS_400} (must be 0)"
        exit 3
    fi

    REQUESTS=$(jq -r '.results.requests // empty' "${META_FILE}")
    if [[ -z "${REQUESTS}" || ! "${REQUESTS}" =~ ^[0-9]+$ || "${REQUESTS}" -le 0 ]]; then
        echo "ERROR: results.requests must be a positive integer for 400/409 gate (got: ${REQUESTS:-<empty>})"
        exit 2
    fi
    VALIDATION_ERROR_RATE=$(awk -v s="${STATUS_400}" -v r="${REQUESTS}" \
        'BEGIN { if (r > 0) printf "%.6f", s / r; else print "0" }')
    CONFLICT_ERROR_RATE_CALCULATED=$(awk -v s="${STATUS_409}" -v r="${REQUESTS}" \
        'BEGIN { if (r > 0) printf "%.6f", s / r; else print "0" }')

    echo "  validation_error_rate (400) = ${VALIDATION_ERROR_RATE}"
    echo "  conflict_error_rate (409) = ${CONFLICT_ERROR_RATE_CALCULATED}"
    echo ""

    # Export for use in threshold checks
    export CONFLICT_ERROR_RATE_CALCULATED
}

# IMPL-TBLR-002: Enforce merge_path_detail fail gate
# Ensures tasks_bulk is using the optimized with_arena merge path
check_merge_path_gate() {
    local MIN_WITH_ARENA_RATIO=0.90
    local BULK_WITH_ARENA
    local BULK_WITHOUT_ARENA
    local BULK_WITH_ARENA_RATIO

    # Check if merge_path_detail exists in meta.json
    if ! jq -e '.results.merge_path_detail' "${META_FILE}" >/dev/null 2>&1; then
        echo "FAIL: .results.merge_path_detail not found in meta.json"
        echo "  merge_path_detail is required for tasks_bulk to ensure with_arena path is used"
        echo "  This indicates profiling is disabled or stacks.folded generation failed"
        exit 3
    fi

    BULK_WITH_ARENA=$(jq -r '.results.merge_path_detail.bulk_with_arena // empty' "${META_FILE}")
    BULK_WITHOUT_ARENA=$(jq -r '.results.merge_path_detail.bulk_without_arena // empty' "${META_FILE}")

    if [[ -z "${BULK_WITH_ARENA}" ]] || [[ -z "${BULK_WITHOUT_ARENA}" ]]; then
        echo "FAIL: merge_path_detail fields incomplete (bulk_with_arena=${BULK_WITH_ARENA}, bulk_without_arena=${BULK_WITHOUT_ARENA})"
        echo "  Both bulk_with_arena and bulk_without_arena must be present"
        echo "  This may indicate merge_path_error occurred or stacks.folded has no merge samples"
        exit 3
    fi

    if [[ ! "${BULK_WITH_ARENA}" =~ ^[0-9]+$ ]] || [[ ! "${BULK_WITHOUT_ARENA}" =~ ^[0-9]+$ ]]; then
        echo "ERROR: merge_path_detail.bulk_* must be non-negative integers (got: bulk_with_arena=${BULK_WITH_ARENA}, bulk_without_arena=${BULK_WITHOUT_ARENA})"
        exit 2
    fi

    # Recalculate ratio from sample counts (do not trust stored ratio)
    local TOTAL=$((BULK_WITH_ARENA + BULK_WITHOUT_ARENA))
    if [[ ${TOTAL} -gt 0 ]]; then
        BULK_WITH_ARENA_RATIO=$(awk -v w="${BULK_WITH_ARENA}" -v t="${TOTAL}" 'BEGIN{printf "%.6f", w/t}')
    else
        BULK_WITH_ARENA_RATIO="0.000000"
    fi

    echo "Merge path telemetry:"
    echo "  bulk_with_arena = ${BULK_WITH_ARENA}"
    echo "  bulk_without_arena = ${BULK_WITHOUT_ARENA}"
    echo "  bulk_with_arena_ratio = ${BULK_WITH_ARENA_RATIO}"
    echo ""

    # Check if bulk_with_arena_ratio meets the threshold
    if awk -v r="${BULK_WITH_ARENA_RATIO}" -v m="${MIN_WITH_ARENA_RATIO}" 'BEGIN{exit (r < m) ? 0 : 1}'; then
        echo "FAIL: Merge path regression detected"
        echo "  bulk_with_arena_ratio = ${BULK_WITH_ARENA_RATIO} (must be >= ${MIN_WITH_ARENA_RATIO})"
        echo "  This indicates tasks_bulk is not using the optimized with_arena path"
        exit 3
    fi

    echo "PASS: Merge path telemetry within acceptable range (ratio >= ${MIN_WITH_ARENA_RATIO})"
    echo ""
}

# IMPL-TBLR-003: Staged regression guard for tasks_bulk
# Prevents regression to pre-optimization performance levels
# Revert thresholds based on Run 21886689088 baseline
check_bulk_regression_guard() {
    local MAX_P99_REVERT=9550
    local MIN_RPS_REVERT=341.36
    local CURRENT_P99
    local CURRENT_RPS

    # P99 is already set as a global variable
    CURRENT_P99="${P99}"

    # Read RPS from meta.json
    CURRENT_RPS=$(jq -r '.results.rps // empty' "${META_FILE}")

    if [[ -z "${CURRENT_RPS}" ]]; then
        echo "FAIL: RPS not found in meta.json"
        echo "  RPS is required for regression guard to prevent performance degradation"
        exit 3
    fi

    echo "Regression guard (tasks_bulk):"
    echo "  p99 = ${CURRENT_P99}ms (revert threshold: <= ${MAX_P99_REVERT}ms)"
    echo "  rps = ${CURRENT_RPS} (revert threshold: >= ${MIN_RPS_REVERT})"
    echo ""

    local P99_VIOLATION=0
    local RPS_VIOLATION=0

    if (( $(echo "${CURRENT_P99} > ${MAX_P99_REVERT}" | bc -l) )); then
        P99_VIOLATION=1
    fi

    if (( $(echo "${CURRENT_RPS} < ${MIN_RPS_REVERT}" | bc -l) )); then
        RPS_VIOLATION=1
    fi

    if [[ ${P99_VIOLATION} -eq 1 ]] || [[ ${RPS_VIOLATION} -eq 1 ]]; then
        echo "FAIL: Regression guard violation detected"
        if [[ ${P99_VIOLATION} -eq 1 ]]; then
            echo "  p99 = ${CURRENT_P99}ms exceeds revert threshold ${MAX_P99_REVERT}ms"
        fi
        if [[ ${RPS_VIOLATION} -eq 1 ]]; then
            echo "  rps = ${CURRENT_RPS} is below revert threshold ${MIN_RPS_REVERT}"
        fi
        echo "  This indicates performance has regressed to pre-optimization levels"
        exit 3
    fi

    echo "PASS: Regression guard within acceptable range"
    echo ""
}

if [[ "${SCENARIO}" == "tasks_update" || "${SCENARIO}" == "tasks_update_steady" || "${SCENARIO}" == "tasks_update_conflict" ]]; then
    check_validation_gate "Contract violation detected - status field included in PUT payload"
elif [[ "${SCENARIO}" == "tasks_update_status" ]]; then
    check_validation_gate "Transition validation error - invalid status transition in PATCH payload"
fi

# Check merge path telemetry for tasks_bulk scenario (IMPL-TBPA2-003)
if [[ "${SCENARIO}" == "tasks_bulk" ]]; then
    check_merge_path_gate
    check_bulk_regression_guard
fi
FAILED=0
FAILURES=""

if (( $(echo "${P50} > ${P50_MAX}" | bc -l) )); then
    FAILURES="${FAILURES}
  - p50=${P50}ms exceeds threshold of ${P50_MAX}ms"
    FAILED=1
fi

if (( $(echo "${P90} > ${P90_MAX}" | bc -l) )); then
    FAILURES="${FAILURES}
  - p90=${P90}ms exceeds threshold of ${P90_MAX}ms"
    FAILED=1
fi

if (( $(echo "${P99} > ${P99_MAX}" | bc -l) )); then
    FAILURES="${FAILURES}
  - p99=${P99}ms exceeds threshold of ${P99_MAX}ms"
    FAILED=1
fi

if [[ -n "${ERROR_RATE_MAX}" ]] && [[ -n "${ERROR_RATE}" ]]; then
    if (( $(echo "${ERROR_RATE} > ${ERROR_RATE_MAX}" | bc -l) )); then
        FAILURES="${FAILURES}
  - error_rate=${ERROR_RATE} exceeds threshold of ${ERROR_RATE_MAX}"
        FAILED=1
    fi
fi

if [[ -n "${CONFLICT_RATE_MAX}" ]]; then
    # Use calculated conflict_error_rate if available (from check_validation_gate), otherwise use .results.conflict_rate
    CONFLICT_RATE_TO_CHECK="${CONFLICT_ERROR_RATE_CALCULATED:-${CONFLICT_RATE}}"
    if [[ -n "${CONFLICT_RATE_TO_CHECK}" ]]; then
        if (( $(echo "${CONFLICT_RATE_TO_CHECK} > ${CONFLICT_RATE_MAX}" | bc -l) )); then
            FAILURES="${FAILURES}
  - conflict_rate=${CONFLICT_RATE_TO_CHECK} exceeds threshold of ${CONFLICT_RATE_MAX}"
            FAILED=1
        fi
    fi
fi
RESULTS_SUMMARY="p50=${P50}ms, p90=${P90}ms, p99=${P99}ms"
THRESHOLDS_SUMMARY="p50<=${P50_MAX}ms, p90<=${P90_MAX}ms, p99<=${P99_MAX}ms"
[[ -n "${ERROR_RATE}" ]] && RESULTS_SUMMARY="${RESULTS_SUMMARY}, error_rate=${ERROR_RATE}"
[[ -n "${ERROR_RATE_MAX}" ]] && THRESHOLDS_SUMMARY="${THRESHOLDS_SUMMARY}, error_rate<=${ERROR_RATE_MAX}"
# Use calculated conflict_error_rate if available
CONFLICT_RATE_FOR_SUMMARY="${CONFLICT_ERROR_RATE_CALCULATED:-${CONFLICT_RATE}}"
[[ -n "${CONFLICT_RATE_FOR_SUMMARY}" ]] && RESULTS_SUMMARY="${RESULTS_SUMMARY}, conflict_rate=${CONFLICT_RATE_FOR_SUMMARY}"
[[ -n "${CONFLICT_RATE_MAX}" ]] && THRESHOLDS_SUMMARY="${THRESHOLDS_SUMMARY}, conflict_rate<=${CONFLICT_RATE_MAX}"
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
