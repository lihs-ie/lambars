#!/usr/bin/env bash
# Test suite for build_phase_metrics_json() in run_benchmark.sh
# Tests: phase-aware metrics generation with profile-aware sustain selection

set -uo pipefail

# Global temp dir for cleanup on EXIT
TEMP_DIR_ROOT=""
cleanup_temp_dirs() {
    if [[ -n "${TEMP_DIR_ROOT:-}" ]]; then
        rm -rf "${TEMP_DIR_ROOT}" 2>/dev/null || true
    fi
}
trap cleanup_temp_dirs EXIT

# Create a unique subdirectory under TEMP_DIR_ROOT for each test case
make_test_tmp_dir() {
    if [[ -z "${TEMP_DIR_ROOT}" ]]; then
        TEMP_DIR_ROOT=$(mktemp -d)
    fi
    mktemp -d "${TEMP_DIR_ROOT}/XXXXXX"
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUN_BENCHMARK_SCRIPT="${SCRIPT_DIR}/../run_benchmark.sh"

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

assert_equals() {
    local expected="$1"
    local actual="$2"
    local test_name="$3"
    ((TESTS_RUN++))
    if [[ "${expected}" == "${actual}" ]]; then
        log_pass "${test_name}"
        ((TESTS_PASSED++))
    else
        log_fail "${test_name}"
        echo "  Expected: ${expected}"
        echo "  Actual:   ${actual}"
        ((TESTS_FAILED++))
        return 1
    fi
}

assert_json_field() {
    local json="$1"
    local field="$2"
    local expected="$3"
    local test_name="$4"
    ((TESTS_RUN++))
    local actual
    actual=$(echo "${json}" | jq -r "${field}" 2>/dev/null)
    if [[ "${expected}" == "${actual}" ]]; then
        log_pass "${test_name}"
        ((TESTS_PASSED++))
    else
        log_fail "${test_name}"
        echo "  Field:    ${field}"
        echo "  Expected: ${expected}"
        echo "  Actual:   ${actual}"
        echo "  JSON:     ${json}"
        ((TESTS_FAILED++))
        return 1
    fi
}

assert_json_null() {
    local json="$1"
    local test_name="$2"
    ((TESTS_RUN++))
    if [[ "${json}" == "null" ]]; then
        log_pass "${test_name}"
        ((TESTS_PASSED++))
    else
        log_fail "${test_name} (expected null, got: ${json})"
        ((TESTS_FAILED++))
        return 1
    fi
}

# -------------------------------------------------------------------
# Load build_phase_metrics_json() from run_benchmark.sh
# -------------------------------------------------------------------

# Source only the function by extracting it and evaluating in subshell
# We avoid sourcing the entire script to prevent side effects
load_build_phase_metrics_json() {
    if [[ ! -f "${RUN_BENCHMARK_SCRIPT}" ]]; then
        echo -e "${RED}ERROR: ${RUN_BENCHMARK_SCRIPT} not found${NC}"
        exit 1
    fi

    # Extract build_phase_metrics_json function body
    # Returns non-zero if function is not found
    if ! grep -q "build_phase_metrics_json" "${RUN_BENCHMARK_SCRIPT}"; then
        return 1
    fi
    return 0
}

# Wrapper to call build_phase_metrics_json() via sourcing run_benchmark.sh
# Temporarily overrides any problematic globals
call_build_phase_metrics_json() {
    local results_dir="$1"
    local profile="${2:-steady}"

    # Source the function in a subshell to avoid polluting the test environment
    (
        # Suppress sourcing side effects by defining dummy guard variables
        # The script checks for these to skip initialization
        export API_URL="${API_URL:-http://localhost:3000}"
        export RESULTS_DIR="${RESULTS_DIR:-/tmp}"
        export SCENARIO_NAME="${SCENARIO_NAME:-test}"
        export STORAGE_MODE="${STORAGE_MODE:-in_memory}"
        export CACHE_MODE="${CACHE_MODE:-none}"
        export DURATION="${DURATION:-30s}"
        export THREADS="${THREADS:-2}"
        export CONNECTIONS="${CONNECTIONS:-10}"
        export RPS_PROFILE="${RPS_PROFILE:-steady}"
        export PROFILE_MODE="${PROFILE_MODE:-false}"

        # Source only the function - use awk to extract the function
        # shellcheck disable=SC1090
        eval "$(awk '/^build_phase_metrics_json\(\)/{found=1} found{print} found && /^\}$/{exit}' "${RUN_BENCHMARK_SCRIPT}")"

        build_phase_metrics_json "${results_dir}" "${profile}"
    )
}

# -------------------------------------------------------------------
# Fixtures
# -------------------------------------------------------------------

create_phase_result() {
    local directory="$1"
    local phase_name="$2"
    local target_rps="$3"
    local actual_rps="$4"
    local duration_seconds="$5"

    mkdir -p "${directory}"
    jq -n \
        --arg phase "${phase_name}" \
        --argjson target_rps "${target_rps}" \
        --argjson actual_rps "${actual_rps}" \
        --argjson duration_seconds "${duration_seconds}" \
        '{
            "phase": $phase,
            "target_rps": $target_rps,
            "actual_rps": $actual_rps,
            "duration_seconds": $duration_seconds
        }' > "${directory}/phase_result.json"
}

# -------------------------------------------------------------------
# Tests
# -------------------------------------------------------------------

test_function_exists() {
    echo ""
    echo "Testing: build_phase_metrics_json exists"
    ((TESTS_RUN++))
    if load_build_phase_metrics_json; then
        log_pass "build_phase_metrics_json() is defined in run_benchmark.sh"
        ((TESTS_PASSED++))
    else
        log_fail "build_phase_metrics_json() is NOT defined in run_benchmark.sh"
        ((TESTS_FAILED++))
    fi
}

# TC-1: steady (single phase) - MERGED_RPS からの fallback
test_steady_single_phase_fallback() {
    echo ""
    echo "TC-1: steady single-phase fallback (MERGED_RPS)"
    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    # No phase_result.json files - steady single-phase scenario
    local result
    result=$(MERGED_RPS="1234.56" call_build_phase_metrics_json "${tmp_dir}" "steady")
    local exit_code=$?

    assert_exit_code "0" "${exit_code}" "TC-1: exit code 0 for single-phase fallback"
    assert_json_field "${result}" ".phase_count" "1" "TC-1: phase_count is 1"
    assert_json_field "${result}" ".peak_phase_rps" "1234.56" "TC-1: peak_phase_rps equals MERGED_RPS"
    assert_json_field "${result}" ".min_phase_rps" "1234.56" "TC-1: min_phase_rps equals MERGED_RPS"
    assert_json_field "${result}" ".weighted_rps" "1234.56" "TC-1: weighted_rps equals MERGED_RPS"
    assert_json_field "${result}" ".sustain_phase_rps" "1234.56" "TC-1: sustain_phase_rps equals MERGED_RPS"
}

# TC-2: burst - 複数フェーズの peak/min/weighted/sustain
test_burst_multiple_phases() {
    echo ""
    echo "TC-2: burst - multiple phases peak/min/weighted/sustain"
    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    # Create burst scenario with 3 phases:
    # - warmup: 100 RPS for 30s
    # - burst: 1000 RPS for 20s
    # - cooldown: 100 RPS for 10s
    create_phase_result "${tmp_dir}/phase_warmup" "warmup" 100 100 30
    create_phase_result "${tmp_dir}/phase_burst" "burst" 1000 1002 20
    create_phase_result "${tmp_dir}/phase_cooldown" "cooldown" 100 98 10

    local result
    result=$(call_build_phase_metrics_json "${tmp_dir}" "burst")
    local exit_code=$?

    assert_exit_code "0" "${exit_code}" "TC-2: exit code 0"
    assert_json_field "${result}" ".phase_count" "3" "TC-2: phase_count is 3"
    assert_json_field "${result}" ".peak_phase_rps" "1002" "TC-2: peak_phase_rps is burst phase value"
    assert_json_field "${result}" ".min_phase_rps" "98" "TC-2: min_phase_rps is cooldown phase value"
    assert_json_field "${result}" ".sustain_phase_rps" "1002" "TC-2: sustain_phase_rps is max of burst-named phases"

    # weighted_rps = (100*30 + 1002*20 + 98*10) / (30+20+10) = (3000+20040+980) / 60 = 24020/60 = 400.333...
    local weighted
    weighted=$(echo "${result}" | jq -r '.weighted_rps')
    # Check it's approximately 400.33 (within 1.0 tolerance)
    ((TESTS_RUN++))
    local diff
    diff=$(awk -v w="${weighted}" 'BEGIN { d = w - 400.333; if (d < 0) d = -d; print (d < 1.0) ? "ok" : "fail" }')
    if [[ "${diff}" == "ok" ]]; then
        log_pass "TC-2: weighted_rps approximately 400.33"
        ((TESTS_PASSED++))
    else
        log_fail "TC-2: weighted_rps expected ~400.33, got ${weighted}"
        ((TESTS_FAILED++))
    fi
}

# TC-3: ramp_up_down - sustain phase の正しい選択
test_ramp_up_down_sustain_phase() {
    echo ""
    echo "TC-3: ramp_up_down - sustain phase selection"
    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    # ramp_up_down: ramp_up, sustain, ramp_down
    create_phase_result "${tmp_dir}/phase_ramp_up" "ramp_up" 500 480 30
    create_phase_result "${tmp_dir}/phase_sustain" "sustain" 500 499 60
    create_phase_result "${tmp_dir}/phase_ramp_down" "ramp_down" 100 102 30

    local result
    result=$(call_build_phase_metrics_json "${tmp_dir}" "ramp_up_down")
    local exit_code=$?

    assert_exit_code "0" "${exit_code}" "TC-3: exit code 0"
    assert_json_field "${result}" ".phase_count" "3" "TC-3: phase_count is 3"
    assert_json_field "${result}" ".sustain_phase_rps" "499" "TC-3: sustain_phase_rps uses 'sustain' phase actual_rps"
    assert_json_field "${result}" ".peak_phase_rps" "499" "TC-3: peak_phase_rps is max actual_rps"
    assert_json_field "${result}" ".min_phase_rps" "102" "TC-3: min_phase_rps is ramp_down value"
}

# TC-4: step_up - 最終ステップの正しい選択
test_step_up_last_step() {
    echo ""
    echo "TC-4: step_up - last step selection"
    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    # step_up: step1, step2, step3 (sorted alphabetically = step1, step2, step3)
    create_phase_result "${tmp_dir}/phase_step1" "step1" 100 98 30
    create_phase_result "${tmp_dir}/phase_step2" "step2" 200 195 30
    create_phase_result "${tmp_dir}/phase_step3" "step3" 300 298 30

    local result
    result=$(call_build_phase_metrics_json "${tmp_dir}" "step_up")
    local exit_code=$?

    assert_exit_code "0" "${exit_code}" "TC-4: exit code 0"
    assert_json_field "${result}" ".phase_count" "3" "TC-4: phase_count is 3"
    assert_json_field "${result}" ".sustain_phase_rps" "298" "TC-4: sustain_phase_rps is last step actual_rps"
    assert_json_field "${result}" ".peak_phase_rps" "298" "TC-4: peak_phase_rps is max actual_rps"
    assert_json_field "${result}" ".min_phase_rps" "98" "TC-4: min_phase_rps is first step value"
}

# TC-5: 空ディレクトリ - MERGED_RPS なし → null を返すこと
test_empty_directory_returns_null() {
    echo ""
    echo "TC-5: empty directory (no MERGED_RPS) returns null"
    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    local result
    result=$(call_build_phase_metrics_json "${tmp_dir}" "steady")
    local exit_code=$?

    assert_exit_code "0" "${exit_code}" "TC-5: exit code 0 for empty dir"
    assert_json_null "${result}" "TC-5: returns null when no files and no MERGED_RPS"
}

# TC-6: weighted_rps の計算精度
test_weighted_rps_calculation_accuracy() {
    echo ""
    echo "TC-6: weighted_rps calculation accuracy"
    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    # Simple case: 2 phases with equal duration
    # phase_a: 200 RPS for 60s
    # phase_b: 400 RPS for 60s
    # weighted_rps = (200*60 + 400*60) / (60+60) = 36000/120 = 300
    create_phase_result "${tmp_dir}/phase_a" "phase_a" 200 200 60
    create_phase_result "${tmp_dir}/phase_b" "phase_b" 400 400 60

    local result
    result=$(call_build_phase_metrics_json "${tmp_dir}" "steady")
    local exit_code=$?

    assert_exit_code "0" "${exit_code}" "TC-6: exit code 0"

    local weighted
    weighted=$(echo "${result}" | jq -r '.weighted_rps')
    ((TESTS_RUN++))
    local diff
    diff=$(awk -v w="${weighted}" 'BEGIN { d = w - 300.0; if (d < 0) d = -d; print (d < 0.01) ? "ok" : "fail" }')
    if [[ "${diff}" == "ok" ]]; then
        log_pass "TC-6: weighted_rps = 300 (exact)"
        ((TESTS_PASSED++))
    else
        log_fail "TC-6: weighted_rps expected 300, got ${weighted}"
        ((TESTS_FAILED++))
    fi
}

# TC-7: steady profile - "main" phase の sustain 選択
test_steady_main_phase_sustain_selection() {
    echo ""
    echo "TC-7: steady - \"main\" phase sustain selection"
    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    create_phase_result "${tmp_dir}/phase_warmup" "warmup" 500 490 10
    create_phase_result "${tmp_dir}/phase_main" "main" 500 502 60

    local result
    result=$(call_build_phase_metrics_json "${tmp_dir}" "steady")
    local exit_code=$?

    assert_exit_code "0" "${exit_code}" "TC-7: exit code 0"
    assert_json_field "${result}" ".sustain_phase_rps" "502" "TC-7: sustain_phase_rps uses 'main' phase actual_rps"
    assert_json_field "${result}" ".peak_phase_rps" "502" "TC-7: peak_phase_rps is max"
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
    echo -e "  test_phase_metrics_generation.sh"
    echo -e "==============================================${NC}"

    if ! command -v jq &>/dev/null; then
        echo -e "${RED}ERROR: jq is required but not installed${NC}"
        exit 1
    fi

    test_function_exists
    test_steady_single_phase_fallback
    test_burst_multiple_phases
    test_ramp_up_down_sustain_phase
    test_step_up_last_step
    test_empty_directory_returns_null
    test_weighted_rps_calculation_accuracy
    test_steady_main_phase_sustain_selection

    print_summary

    if [[ ${TESTS_FAILED} -gt 0 ]]; then
        exit 1
    fi
}

main "$@"
