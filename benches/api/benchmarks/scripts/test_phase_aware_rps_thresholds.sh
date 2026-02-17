#!/usr/bin/env bash
# Test suite for phase-aware RPS threshold evaluation in check_thresholds.sh
# Tests: RPS threshold checking using phase_metrics from meta.json
#
# Test cases:
#   TC-RPS-1: RPS >= warning threshold -> PASS
#   TC-RPS-2: RPS between warning and error -> WARNING (exit 0)
#   TC-RPS-3: RPS < error threshold -> FAIL (exit 3)
#   TC-RPS-4: phase_metrics absent (legacy meta.json) -> fallback to merged RPS
#   TC-RPS-5: rps rule not defined in thresholds.yaml -> skip (PASS)
#   TC-RPS-6: each profile uses correct metric (steady/burst/ramp/step)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK_THRESHOLDS_SCRIPT="${SCRIPT_DIR}/../check_thresholds.sh"
THRESHOLDS_YAML="${SCRIPT_DIR}/../thresholds.yaml"

TEMP_DIR_ROOT=""
cleanup_temp_dirs() {
    if [[ -n "${TEMP_DIR_ROOT:-}" ]]; then
        rm -rf "${TEMP_DIR_ROOT}" 2>/dev/null || true
    fi
}
trap cleanup_temp_dirs EXIT

make_test_tmp_dir() {
    if [[ -z "${TEMP_DIR_ROOT}" ]]; then
        TEMP_DIR_ROOT=$(mktemp -d)
    fi
    mktemp -d "${TEMP_DIR_ROOT}/XXXXXX"
}

TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

log_test() { echo -e "${BLUE}[TEST]${NC} $1"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }

assert_exit_code() {
    local expected="$1"
    local actual="$2"
    local test_name="$3"
    ((TESTS_RUN++))
    if [[ "${expected}" == "${actual}" ]]; then
        log_pass "${test_name}"
        ((TESTS_PASSED++))
    else
        log_fail "${test_name} (expected exit=${expected}, got exit=${actual})"
        ((TESTS_FAILED++))
        return 1
    fi
}

assert_contains() {
    local output="$1"
    local pattern="$2"
    local test_name="$3"
    ((TESTS_RUN++))
    if echo "${output}" | grep -q "${pattern}"; then
        log_pass "${test_name}"
        ((TESTS_PASSED++))
    else
        log_fail "${test_name} (pattern '${pattern}' not found in output)"
        echo "  Output: ${output}"
        ((TESTS_FAILED++))
        return 1
    fi
}

assert_not_contains() {
    local output="$1"
    local pattern="$2"
    local test_name="$3"
    ((TESTS_RUN++))
    if ! echo "${output}" | grep -q "${pattern}"; then
        log_pass "${test_name}"
        ((TESTS_PASSED++))
    else
        log_fail "${test_name} (unexpected pattern '${pattern}' found in output)"
        echo "  Output: ${output}"
        ((TESTS_FAILED++))
        return 1
    fi
}

# Create a minimal valid meta.json for tasks_bulk scenario with phase_metrics
# merged_rps: used by regression guard (results.rps) - must be >= 341.36 to pass guard
# weighted_rps: used by RPS threshold check via phase_metrics.weighted_rps
make_meta_json_with_phase_metrics() {
    local directory="$1"
    local weighted_rps="$2"
    local peak_phase_rps="${3:-${weighted_rps}}"
    local sustain_phase_rps="${4:-${weighted_rps}}"
    # merged_rps for results.rps (regression guard): default to 500 to pass guard
    local merged_rps="${5:-500}"

    mkdir -p "${directory}/tasks_bulk/benchmark/meta"
    jq -n \
        --argjson weighted_rps "${weighted_rps}" \
        --argjson peak_phase_rps "${peak_phase_rps}" \
        --argjson sustain_phase_rps "${sustain_phase_rps}" \
        --argjson merged_rps "${merged_rps}" \
        '{
            "scenario": "tasks_bulk",
            "results": {
                "rps": $merged_rps,
                "p50": "5ms",
                "p90": "10ms",
                "p99": "20ms",
                "error_rate": 0.001,
                "requests": 1000,
                "http_status": {
                    "200": 999,
                    "400": 0,
                    "409": 0
                },
                "merge_path_detail": {
                    "bulk_with_arena": 950,
                    "bulk_without_arena": 50
                },
                "phase_metrics": {
                    "peak_phase_rps": $peak_phase_rps,
                    "sustain_phase_rps": $sustain_phase_rps,
                    "weighted_rps": $weighted_rps,
                    "min_phase_rps": ($weighted_rps * 0.8),
                    "phase_count": 1
                }
            }
        }' > "${directory}/tasks_bulk/benchmark/meta/tasks_bulk.json"
}

# Create a minimal valid meta.json WITHOUT phase_metrics (legacy format)
make_meta_json_without_phase_metrics() {
    local directory="$1"
    local merged_rps="$2"

    mkdir -p "${directory}/tasks_bulk/benchmark/meta"
    jq -n \
        --argjson rps "${merged_rps}" \
        '{
            "scenario": "tasks_bulk",
            "results": {
                "rps": $rps,
                "p50": "5ms",
                "p90": "10ms",
                "p99": "20ms",
                "error_rate": 0.001,
                "requests": 1000,
                "http_status": {
                    "200": 999,
                    "400": 0,
                    "409": 0
                },
                "merge_path_detail": {
                    "bulk_with_arena": 950,
                    "bulk_without_arena": 50
                }
            }
        }' > "${directory}/tasks_bulk/benchmark/meta/tasks_bulk.json"
}

# -------------------------------------------------------------------
# TC-RPS-1: RPS が warning 閾値以上 -> PASS
# -------------------------------------------------------------------
test_rps_above_warning_threshold_is_pass() {
    log_test "TC-RPS-1: RPS >= warning threshold -> PASS"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    # tasks_bulk: warning=425, error=350
    # RPS = 500 (above warning) -> should PASS
    make_meta_json_with_phase_metrics "${tmp_dir}" 500

    local output
    local exit_code
    output=$("${CHECK_THRESHOLDS_SCRIPT}" "${tmp_dir}" "tasks_bulk" 2>&1) || exit_code=$?
    exit_code="${exit_code:-0}"

    assert_exit_code "0" "${exit_code}" "TC-RPS-1: exit code 0"
    assert_contains "${output}" "PASS" "TC-RPS-1: output contains PASS"
    assert_not_contains "${output}" "RPS.*FAIL\|FAIL.*RPS\|RPS.*WARNING\|WARNING.*RPS" "TC-RPS-1: no RPS failure or warning"
}

# -------------------------------------------------------------------
# TC-RPS-2: RPS が warning と error の間 -> WARNING (exit 0)
# -------------------------------------------------------------------
test_rps_between_warning_and_error_is_warning() {
    log_test "TC-RPS-2: warning < RPS < error -> WARNING (exit 0)"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    # tasks_bulk: warning=425, error=350
    # RPS = 400 (between error=350 and warning=425) -> should WARNING
    make_meta_json_with_phase_metrics "${tmp_dir}" 400

    local output
    local exit_code
    output=$("${CHECK_THRESHOLDS_SCRIPT}" "${tmp_dir}" "tasks_bulk" 2>&1) || exit_code=$?
    exit_code="${exit_code:-0}"

    assert_exit_code "0" "${exit_code}" "TC-RPS-2: exit code 0 (not fail)"
    assert_contains "${output}" "WARNING" "TC-RPS-2: output contains WARNING"
    assert_not_contains "${output}" "RPS.*FAIL\|FAIL.*RPS" "TC-RPS-2: no RPS FAIL"
}

# -------------------------------------------------------------------
# TC-RPS-3: RPS が error 閾値未満 -> FAIL (exit 3)
# -------------------------------------------------------------------
test_rps_below_error_threshold_is_fail() {
    log_test "TC-RPS-3: RPS < error threshold -> FAIL (exit 3)"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    # tasks_bulk: warning=425, error=350
    # weighted_rps = 200 (below error=350) -> should FAIL with RPS message
    # merged_rps (results.rps) = 500 to pass regression guard (>= 341.36)
    make_meta_json_with_phase_metrics "${tmp_dir}" 200 200 200 500

    local output
    local exit_code=0
    output=$("${CHECK_THRESHOLDS_SCRIPT}" "${tmp_dir}" "tasks_bulk" 2>&1) || exit_code=$?

    assert_exit_code "3" "${exit_code}" "TC-RPS-3: exit code 3"
    assert_contains "${output}" "FAIL" "TC-RPS-3: output contains FAIL"
    assert_contains "${output}" "weighted_rps" "TC-RPS-3: FAIL message mentions weighted_rps metric"
}

# -------------------------------------------------------------------
# TC-RPS-4: phase_metrics がない旧形式 meta.json -> merged RPS で fallback
# -------------------------------------------------------------------
test_legacy_meta_json_falls_back_to_merged_rps() {
    log_test "TC-RPS-4: phase_metrics absent -> fallback to merged RPS"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    # Legacy meta.json without phase_metrics
    # merged RPS = 500 (above warning=425) -> should PASS
    make_meta_json_without_phase_metrics "${tmp_dir}" 500

    local output
    local exit_code
    output=$("${CHECK_THRESHOLDS_SCRIPT}" "${tmp_dir}" "tasks_bulk" 2>&1) || exit_code=$?
    exit_code="${exit_code:-0}"

    assert_exit_code "0" "${exit_code}" "TC-RPS-4: exit code 0 with legacy meta"
    assert_contains "${output}" "PASS" "TC-RPS-4: PASS with fallback to merged RPS"
}

# -------------------------------------------------------------------
# TC-RPS-5: RPS ルールが thresholds.yaml に未定義 -> スキップ (PASS)
# -------------------------------------------------------------------
test_no_rps_rule_skips_check() {
    log_test "TC-RPS-5: rps rule not in thresholds.yaml -> skip (PASS)"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    # Create meta.json for tasks_search_hot with high RPS
    mkdir -p "${tmp_dir}/tasks_search_hot/benchmark/meta"
    jq -n '{
        "scenario": "tasks_search_hot",
        "results": {
            "rps": 2000,
            "p50": "10ms",
            "p90": "50ms",
            "p99": "100ms",
            "error_rate": 0.001,
            "requests": 5000,
            "http_status": {
                "200": 4995,
                "400": 0,
                "409": 0
            }
        }
    }' > "${tmp_dir}/tasks_search_hot/benchmark/meta/tasks_search_hot.json"

    local output
    local exit_code
    output=$("${CHECK_THRESHOLDS_SCRIPT}" "${tmp_dir}" "tasks_search_hot" 2>&1) || exit_code=$?
    exit_code="${exit_code:-0}"

    assert_exit_code "0" "${exit_code}" "TC-RPS-5: exit code 0 (RPS check skipped or passed)"
    assert_contains "${output}" "PASS" "TC-RPS-5: PASS when RPS rule absent or met"
}

# -------------------------------------------------------------------
# TC-RPS-6: 各 profile で正しい metric が使われること
# -------------------------------------------------------------------
test_each_profile_uses_correct_metric() {
    log_test "TC-RPS-6: constant profile uses weighted_rps (not peak_phase_rps)"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    # weighted_rps=200 (below error=350), peak=600 (above warning=425)
    # merged_rps (results.rps) = 500 to pass regression guard (>= 341.36)
    make_meta_json_with_phase_metrics "${tmp_dir}" 200 600 200 500

    local output
    local exit_code=0
    output=$("${CHECK_THRESHOLDS_SCRIPT}" "${tmp_dir}" "tasks_bulk" 2>&1) || exit_code=$?

    assert_exit_code "3" "${exit_code}" "TC-RPS-6: exit code 3 (uses weighted_rps not peak)"
    assert_contains "${output}" "FAIL" "TC-RPS-6: FAIL when weighted_rps is below error threshold"
    assert_contains "${output}" "weighted_rps" "TC-RPS-6: FAIL message references weighted_rps metric"
}

# -------------------------------------------------------------------
# Summary
# -------------------------------------------------------------------

print_summary() {
    echo ""
    echo -e "${BOLD}=============================================="
    echo -e "  Test Summary"
    echo -e "==============================================${NC}"
    echo -e "  Total:  ${TESTS_RUN}"
    echo -e "  ${GREEN}Passed: ${TESTS_PASSED}${NC}"
    if [[ ${TESTS_FAILED} -gt 0 ]]; then
        echo -e "  ${RED}Failed: ${TESTS_FAILED}${NC}"
    else
        echo -e "  Failed: ${TESTS_FAILED}"
    fi
    echo ""
}

# -------------------------------------------------------------------
# Main
# -------------------------------------------------------------------

main() {
    echo -e "${BOLD}=============================================="
    echo -e "  test_phase_aware_rps_thresholds.sh"
    echo -e "==============================================${NC}"

    for cmd in jq yq; do
        if ! command -v "${cmd}" &>/dev/null; then
            echo -e "${RED}ERROR: ${cmd} is required but not installed${NC}"
            exit 1
        fi
    done

    if [[ ! -f "${CHECK_THRESHOLDS_SCRIPT}" ]]; then
        echo -e "${RED}ERROR: ${CHECK_THRESHOLDS_SCRIPT} not found${NC}"
        exit 1
    fi

    if [[ ! -f "${THRESHOLDS_YAML}" ]]; then
        echo -e "${RED}ERROR: ${THRESHOLDS_YAML} not found${NC}"
        exit 1
    fi

    test_rps_above_warning_threshold_is_pass
    test_rps_between_warning_and_error_is_warning
    test_rps_below_error_threshold_is_fail
    test_legacy_meta_json_falls_back_to_merged_rps
    test_no_rps_rule_skips_check
    test_each_profile_uses_correct_metric

    print_summary

    if [[ ${TESTS_FAILED} -gt 0 ]]; then
        exit 1
    fi
}

main "$@"
