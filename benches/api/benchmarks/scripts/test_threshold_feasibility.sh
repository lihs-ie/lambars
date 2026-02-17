#!/usr/bin/env bash
# Test suite for validate_threshold_feasibility.sh
# Tests: Theoretical upper bound calculation per profile type
#
# Test cases:
#   TC-FEAS-1: steady: target=1000, threshold=900 -> PASS (達成可能)
#   TC-FEAS-2: steady: target=100, threshold=500 -> FAIL (達成不可能)
#   TC-FEAS-3: burst: target=1000, burst_multiplier=2, metric=peak_phase_rps, threshold=900 -> PASS
#   TC-FEAS-4: burst: target=100, metric=weighted_rps, threshold=500 -> FAIL (理論上限超過)
#   TC-FEAS-5: step_up: steps=[100,200,300], metric=weighted_rps, threshold=250 -> FAIL (weighted上限=200)
#   TC-FEAS-6: ramp_up_down: target=1000, metric=sustain_phase_rps, threshold=900 -> PASS
#   TC-FEAS-7: RPS ルール未定義 -> スキップ (PASS)
#   TC-FEAS-8: --mode warn: 達成不可能でも exit 0
#   TC-FEAS-9: --mode strict: 達成不可能で exit 1

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VALIDATE_SCRIPT="${SCRIPT_DIR}/validate_threshold_feasibility.sh"

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

# YAML コンテンツをファイルに書き込む (シナリオ・閾値ファイル共用)
write_yaml_file() {
    echo "${2}" > "${1}"
}
make_scenario_yaml()   { write_yaml_file "$@"; }
make_thresholds_yaml() { write_yaml_file "$@"; }

# -------------------------------------------------------------------
# TC-FEAS-1: steady: target=1000, threshold=900 -> PASS (達成可能)
# -------------------------------------------------------------------
test_steady_feasible_threshold_is_pass() {
    log_test "TC-FEAS-1: steady target=1000, threshold=900 -> PASS"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    local scenario_file="${tmp_dir}/scenario.yaml"
    make_scenario_yaml "${scenario_file}" "name: test_steady_pass
rps_profile: steady
target_rps: 1000"

    local threshold_file="${tmp_dir}/thresholds.yaml"
    make_thresholds_yaml "${threshold_file}" "scenarios:
  test_steady_pass:
    rps:
      metric: weighted_rps
      warning: 850
      error: 900"

    local output
    local exit_code
    output=$(bash "${VALIDATE_SCRIPT}" \
        --scenario-file "${scenario_file}" \
        --threshold-file "${threshold_file}" \
        2>&1) || exit_code=$?
    exit_code="${exit_code:-0}"

    assert_exit_code "0" "${exit_code}" "TC-FEAS-1: exit code 0"
    assert_contains "${output}" "PASS" "TC-FEAS-1: output contains PASS"
    assert_not_contains "${output}" "INFEASIBLE\|FAIL" "TC-FEAS-1: no INFEASIBLE or FAIL"
}

# -------------------------------------------------------------------
# TC-FEAS-2: steady: target=100, threshold=500 -> FAIL (達成不可能)
# -------------------------------------------------------------------
test_steady_infeasible_threshold_is_fail() {
    log_test "TC-FEAS-2: steady target=100, threshold=500 -> FAIL (infeasible)"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    local scenario_file="${tmp_dir}/scenario.yaml"
    make_scenario_yaml "${scenario_file}" "name: test_steady_fail
rps_profile: steady
target_rps: 100"

    local threshold_file="${tmp_dir}/thresholds.yaml"
    make_thresholds_yaml "${threshold_file}" "scenarios:
  test_steady_fail:
    rps:
      metric: weighted_rps
      warning: 400
      error: 500"

    local output
    local exit_code=0
    output=$(bash "${VALIDATE_SCRIPT}" \
        --scenario-file "${scenario_file}" \
        --threshold-file "${threshold_file}" \
        --mode strict \
        2>&1) || exit_code=$?

    assert_exit_code "1" "${exit_code}" "TC-FEAS-2: exit code 1 (strict mode)"
    assert_contains "${output}" "FAIL\|INFEASIBLE" "TC-FEAS-2: output contains FAIL or INFEASIBLE"
    assert_contains "${output}" "test_steady_fail" "TC-FEAS-2: output contains scenario name"
}

# -------------------------------------------------------------------
# TC-FEAS-3: burst: target=1000, burst_multiplier=2, metric=peak_phase_rps, threshold=900 -> PASS
# -------------------------------------------------------------------
test_burst_peak_phase_rps_feasible_is_pass() {
    log_test "TC-FEAS-3: burst peak_phase_rps=1000, threshold=900 -> PASS"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    local scenario_file="${tmp_dir}/scenario.yaml"
    make_scenario_yaml "${scenario_file}" "name: test_burst_peak_pass
rps_profile: burst
target_rps: 1000
burst_multiplier: 2.0
burst_duration_seconds: 5
burst_interval_seconds: 20"

    local threshold_file="${tmp_dir}/thresholds.yaml"
    make_thresholds_yaml "${threshold_file}" "scenarios:
  test_burst_peak_pass:
    rps:
      metric: peak_phase_rps
      warning: 850
      error: 900"

    local output
    local exit_code
    output=$(bash "${VALIDATE_SCRIPT}" \
        --scenario-file "${scenario_file}" \
        --threshold-file "${threshold_file}" \
        2>&1) || exit_code=$?
    exit_code="${exit_code:-0}"

    assert_exit_code "0" "${exit_code}" "TC-FEAS-3: exit code 0"
    assert_contains "${output}" "PASS" "TC-FEAS-3: output contains PASS"
    assert_not_contains "${output}" "INFEASIBLE\|FAIL" "TC-FEAS-3: no INFEASIBLE or FAIL"
}

# -------------------------------------------------------------------
# TC-FEAS-4: burst: target=100, metric=weighted_rps, threshold=500 -> FAIL (理論上限超過)
# -------------------------------------------------------------------
test_burst_weighted_rps_infeasible_is_fail() {
    log_test "TC-FEAS-4: burst weighted_rps upper_bound < threshold=500 -> FAIL"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    local scenario_file="${tmp_dir}/scenario.yaml"
    # target_rps=100, burst_multiplier=3
    # base_rps = 100/3 ≈ 33.3
    # burst_ratio = 5/20 = 0.25
    # weighted_rps = 0.25 * 100 + 0.75 * 33.3 ≈ 50
    make_scenario_yaml "${scenario_file}" "name: test_burst_weighted_fail
rps_profile: burst
target_rps: 100
burst_multiplier: 3.0
burst_duration_seconds: 5
burst_interval_seconds: 20"

    local threshold_file="${tmp_dir}/thresholds.yaml"
    make_thresholds_yaml "${threshold_file}" "scenarios:
  test_burst_weighted_fail:
    rps:
      metric: weighted_rps
      warning: 400
      error: 500"

    local output
    local exit_code=0
    output=$(bash "${VALIDATE_SCRIPT}" \
        --scenario-file "${scenario_file}" \
        --threshold-file "${threshold_file}" \
        --mode strict \
        2>&1) || exit_code=$?

    assert_exit_code "1" "${exit_code}" "TC-FEAS-4: exit code 1 (strict mode)"
    assert_contains "${output}" "FAIL\|INFEASIBLE" "TC-FEAS-4: output contains FAIL or INFEASIBLE"
    assert_contains "${output}" "test_burst_weighted_fail" "TC-FEAS-4: output contains scenario name"
}

# -------------------------------------------------------------------
# TC-FEAS-5: step_up: steps=[100,200,300], metric=weighted_rps, threshold=250 -> FAIL
# -------------------------------------------------------------------
test_step_up_weighted_rps_infeasible_is_fail() {
    log_test "TC-FEAS-5: step_up steps=[100,200,300] weighted_rps=200, threshold=250 -> FAIL"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    local scenario_file="${tmp_dir}/scenario.yaml"
    # min_rps=100, target_rps=300, step_count=3, duration=90s
    # step_duration = 90/3 = 30s
    # step1=100 (30s), step2=200 (30s), step3=300 (30s)
    # weighted_rps = (100*30 + 200*30 + 300*30) / 90 = 18000/90 = 200
    make_scenario_yaml "${scenario_file}" "name: test_step_up_fail
rps_profile: step_up
target_rps: 300
min_rps: 100
step_count: 3
duration_seconds: 90"

    local threshold_file="${tmp_dir}/thresholds.yaml"
    make_thresholds_yaml "${threshold_file}" "scenarios:
  test_step_up_fail:
    rps:
      metric: weighted_rps
      warning: 240
      error: 250"

    local output
    local exit_code=0
    output=$(bash "${VALIDATE_SCRIPT}" \
        --scenario-file "${scenario_file}" \
        --threshold-file "${threshold_file}" \
        --mode strict \
        2>&1) || exit_code=$?

    assert_exit_code "1" "${exit_code}" "TC-FEAS-5: exit code 1 (strict mode)"
    assert_contains "${output}" "FAIL\|INFEASIBLE" "TC-FEAS-5: output contains FAIL or INFEASIBLE"
    assert_contains "${output}" "test_step_up_fail" "TC-FEAS-5: output contains scenario name"
}

# -------------------------------------------------------------------
# TC-FEAS-6: ramp_up_down: target=1000, metric=sustain_phase_rps, threshold=900 -> PASS
# -------------------------------------------------------------------
test_ramp_up_down_sustain_phase_rps_feasible_is_pass() {
    log_test "TC-FEAS-6: ramp_up_down sustain_phase_rps=1000, threshold=900 -> PASS"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    local scenario_file="${tmp_dir}/scenario.yaml"
    make_scenario_yaml "${scenario_file}" "name: test_ramp_up_down_pass
rps_profile: ramp_up_down
target_rps: 1000"

    local threshold_file="${tmp_dir}/thresholds.yaml"
    make_thresholds_yaml "${threshold_file}" "scenarios:
  test_ramp_up_down_pass:
    rps:
      metric: sustain_phase_rps
      warning: 850
      error: 900"

    local output
    local exit_code
    output=$(bash "${VALIDATE_SCRIPT}" \
        --scenario-file "${scenario_file}" \
        --threshold-file "${threshold_file}" \
        2>&1) || exit_code=$?
    exit_code="${exit_code:-0}"

    assert_exit_code "0" "${exit_code}" "TC-FEAS-6: exit code 0"
    assert_contains "${output}" "PASS" "TC-FEAS-6: output contains PASS"
    assert_not_contains "${output}" "INFEASIBLE\|FAIL" "TC-FEAS-6: no INFEASIBLE or FAIL"
}

# -------------------------------------------------------------------
# TC-FEAS-7: RPS ルール未定義 -> スキップ (PASS)
# -------------------------------------------------------------------
test_no_rps_rule_skips_check() {
    log_test "TC-FEAS-7: RPS ルール未定義 -> スキップ (PASS)"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    local scenario_file="${tmp_dir}/scenario.yaml"
    make_scenario_yaml "${scenario_file}" "name: test_no_rps_rule
rps_profile: steady
target_rps: 100"

    local threshold_file="${tmp_dir}/thresholds.yaml"
    make_thresholds_yaml "${threshold_file}" "scenarios:
  test_no_rps_rule:
    p99_latency_ms:
      warning: 100
      error: 200"

    local output
    local exit_code
    output=$(bash "${VALIDATE_SCRIPT}" \
        --scenario-file "${scenario_file}" \
        --threshold-file "${threshold_file}" \
        2>&1) || exit_code=$?
    exit_code="${exit_code:-0}"

    assert_exit_code "0" "${exit_code}" "TC-FEAS-7: exit code 0 (skipped)"
    assert_contains "${output}" "PASS\|SKIP" "TC-FEAS-7: output contains PASS or SKIP"
    assert_not_contains "${output}" "FAIL\|INFEASIBLE" "TC-FEAS-7: no FAIL or INFEASIBLE"
}

# -------------------------------------------------------------------
# TC-FEAS-8: --mode warn: 達成不可能でも exit 0
# -------------------------------------------------------------------
test_warn_mode_infeasible_is_exit_0() {
    log_test "TC-FEAS-8: --mode warn: infeasible -> exit 0"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    local scenario_file="${tmp_dir}/scenario.yaml"
    make_scenario_yaml "${scenario_file}" "name: test_warn_mode
rps_profile: steady
target_rps: 100"

    local threshold_file="${tmp_dir}/thresholds.yaml"
    make_thresholds_yaml "${threshold_file}" "scenarios:
  test_warn_mode:
    rps:
      metric: weighted_rps
      warning: 400
      error: 500"

    local output
    local exit_code
    output=$(bash "${VALIDATE_SCRIPT}" \
        --scenario-file "${scenario_file}" \
        --threshold-file "${threshold_file}" \
        --mode warn \
        2>&1) || exit_code=$?
    exit_code="${exit_code:-0}"

    assert_exit_code "0" "${exit_code}" "TC-FEAS-8: exit code 0 (warn mode)"
    assert_contains "${output}" "FAIL\|WARN\|INFEASIBLE" "TC-FEAS-8: output contains FAIL/WARN/INFEASIBLE message"
}

# -------------------------------------------------------------------
# TC-FEAS-9: --mode strict: 達成不可能で exit 1
# -------------------------------------------------------------------
test_strict_mode_infeasible_is_exit_1() {
    log_test "TC-FEAS-9: --mode strict: infeasible -> exit 1"

    local tmp_dir
    tmp_dir=$(make_test_tmp_dir)

    local scenario_file="${tmp_dir}/scenario.yaml"
    make_scenario_yaml "${scenario_file}" "name: test_strict_mode
rps_profile: steady
target_rps: 100"

    local threshold_file="${tmp_dir}/thresholds.yaml"
    make_thresholds_yaml "${threshold_file}" "scenarios:
  test_strict_mode:
    rps:
      metric: weighted_rps
      warning: 400
      error: 500"

    local output
    local exit_code=0
    output=$(bash "${VALIDATE_SCRIPT}" \
        --scenario-file "${scenario_file}" \
        --threshold-file "${threshold_file}" \
        --mode strict \
        2>&1) || exit_code=$?

    assert_exit_code "1" "${exit_code}" "TC-FEAS-9: exit code 1 (strict mode)"
    assert_contains "${output}" "FAIL\|INFEASIBLE" "TC-FEAS-9: output contains FAIL or INFEASIBLE"
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
    echo -e "  test_threshold_feasibility.sh"
    echo -e "==============================================${NC}"

    for cmd in yq; do
        if ! command -v "${cmd}" &>/dev/null; then
            echo -e "${RED}ERROR: ${cmd} is required but not installed${NC}"
            exit 1
        fi
    done

    if [[ ! -f "${VALIDATE_SCRIPT}" ]]; then
        echo -e "${RED}ERROR: ${VALIDATE_SCRIPT} not found${NC}"
        exit 1
    fi

    test_steady_feasible_threshold_is_pass
    test_steady_infeasible_threshold_is_fail
    test_burst_peak_phase_rps_feasible_is_pass
    test_burst_weighted_rps_infeasible_is_fail
    test_step_up_weighted_rps_infeasible_is_fail
    test_ramp_up_down_sustain_phase_rps_feasible_is_pass
    test_no_rps_rule_skips_check
    test_warn_mode_infeasible_is_exit_0
    test_strict_mode_infeasible_is_exit_1

    print_summary

    if [[ ${TESTS_FAILED} -gt 0 ]]; then
        exit 1
    fi
}

main "$@"
