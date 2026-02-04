#!/bin/bash
# Test HTTP status collection pipeline
#
# Usage:
#   ./test_http_status_pipeline.sh           # Check existing results only
#   ./test_http_status_pipeline.sh --run     # Run benchmark and then check results
#   ./test_http_status_pipeline.sh --help    # Show help
#
# REQ-PIPELINE-007: Integration tests for HTTP status collection pipeline

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_DIR="$(dirname "${SCRIPT_DIR}")"

TESTS_PASSED=0
TESTS_FAILED=0
RUN_BENCHMARK=false

log_test() { echo -e "${CYAN}[TEST]${NC} $*"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $*"; TESTS_PASSED=$((TESTS_PASSED + 1)); }
log_fail() { echo -e "${RED}[FAIL]${NC} $*"; TESTS_FAILED=$((TESTS_FAILED + 1)); }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_info() { echo -e "${CYAN}[INFO]${NC} $*"; }

show_help() {
    cat << EOF
HTTP Status Collection Pipeline Test Suite

Usage:
    $(basename "$0") [OPTIONS]

Options:
    --run       Run benchmark before testing (requires API server)
    --help      Show this help message

Description:
    This script tests the HTTP status collection pipeline (REQ-PIPELINE-007).

    Without --run flag:
        Tests existing benchmark results in ${BENCH_DIR}/results

    With --run flag:
        1. Runs a quick benchmark (tasks_bulk, 5 seconds)
        2. Validates the generated files
        3. Checks pipeline consistency

Requirements:
    - jq (for JSON parsing)
    - wrk (if using --run flag)
    - API server running on localhost:8080 (if using --run flag)

Examples:
    # Check existing results
    ./$(basename "$0")

    # Run benchmark and check results
    ./$(basename "$0") --run
EOF
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --run)
                RUN_BENCHMARK=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
                show_help
                exit 1
                ;;
        esac
    done
}

run_quick_benchmark() {
    log_info "Running quick benchmark (tasks_bulk, 5 seconds)..."

    if ! command -v wrk &> /dev/null; then
        log_fail "wrk is required but not installed"
        return 1
    fi

    # Check if API server is running
    if ! curl -s --connect-timeout 2 "http://localhost:8080/health" &>/dev/null; then
        log_fail "API server not running on localhost:8080"
        log_info "Start the server with: cargo run --release --bin api_server"
        return 1
    fi

    cd "${BENCH_DIR}"
    if ./run_benchmark.sh --scenario tasks_bulk --quick 2>&1 | tail -20; then
        log_pass "Benchmark completed successfully"
        return 0
    else
        log_fail "Benchmark failed"
        return 1
    fi
}

find_latest_results() {
    find "${BENCH_DIR}/results" -type d -name "20*" 2>/dev/null | sort -r | head -1
}

test_lua_metrics_exists() {
    log_test "Checking lua_metrics.json existence..."

    local results_dir lua_metrics
    results_dir=$(find_latest_results)

    if [[ -z "${results_dir}" ]]; then
        log_fail "No results directory found"
        return 1
    fi

    # Find lua_metrics.json: check both root and script-specific subdirectories
    if [[ -f "${results_dir}/lua_metrics.json" ]]; then
        lua_metrics="${results_dir}/lua_metrics.json"
    else
        # Look for lua_metrics.json in subdirectories (e.g., tasks_bulk/lua_metrics.json)
        lua_metrics=$(find "${results_dir}" -name "lua_metrics.json" -type f 2>/dev/null | head -1)
    fi

    if [[ -z "${lua_metrics}" || ! -f "${lua_metrics}" ]]; then
        log_fail "lua_metrics.json not found in ${results_dir}"
        return 1
    fi

    log_pass "lua_metrics.json exists at ${lua_metrics}"
}

test_http_status_not_empty() {
    log_test "Checking http_status is not empty..."

    local results_dir lua_metrics http_status status_count
    results_dir=$(find_latest_results)

    if [[ -z "${results_dir}" ]]; then
        log_fail "No results directory found"
        return 1
    fi

    # Find lua_metrics.json: check both root and script-specific subdirectories
    if [[ -f "${results_dir}/lua_metrics.json" ]]; then
        lua_metrics="${results_dir}/lua_metrics.json"
    else
        lua_metrics=$(find "${results_dir}" -name "lua_metrics.json" -type f 2>/dev/null | head -1 || true)
    fi

    if [[ -z "${lua_metrics}" || ! -f "${lua_metrics}" ]]; then
        log_fail "lua_metrics.json not found"
        return 1
    fi

    http_status=$(jq -c '.http_status' "${lua_metrics}" 2>/dev/null || echo "{}")
    status_count=$(jq 'length' <<< "${http_status}" 2>/dev/null || echo "0")

    if [[ "${http_status}" == "{}" || "${http_status}" == "null" || "${status_count}" == "0" ]]; then
        log_fail "http_status is empty or has no entries"
        return 1
    fi

    log_pass "http_status has ${status_count} status codes"
    echo "    http_status: ${http_status}"
}

test_meta_http_status_match() {
    log_test "Checking meta.json http_status matches lua_metrics..."

    local results_dir lua_metrics meta_json lua_http_status meta_http_status
    results_dir=$(find_latest_results)

    if [[ -z "${results_dir}" ]]; then
        log_fail "No results directory found"
        return 1
    fi

    # Find both files in the same directory for consistency
    # Exclude phase_* directories and find directories with BOTH meta.json and lua_metrics.json
    # Priority: script-specific subdirectory > root
    local found_dir=""

    # Check subdirectories first (exclude phase_* and results_dir itself)
    local script_dirs
    script_dirs=$(find "${results_dir}" -maxdepth 1 -type d ! -name "$(basename "${results_dir}")" ! -name "phase_*" 2>/dev/null || true)

    if [[ -n "${script_dirs}" ]]; then
        # Find first directory with BOTH meta.json and lua_metrics.json
        while IFS= read -r dir; do
            [[ -z "${dir}" ]] && continue
            if [[ -f "${dir}/meta.json" && -f "${dir}/lua_metrics.json" ]]; then
                found_dir="${dir}"
                break
            fi
        done <<< "${script_dirs}"
    fi

    # Fallback to root if both files exist there
    if [[ -z "${found_dir}" ]] && [[ -f "${results_dir}/meta.json" && -f "${results_dir}/lua_metrics.json" ]]; then
        found_dir="${results_dir}"
    fi

    if [[ -z "${found_dir}" ]]; then
        log_fail "No directory with both meta.json and lua_metrics.json found in ${results_dir}"
        return 1
    fi

    lua_metrics="${found_dir}/lua_metrics.json"
    meta_json="${found_dir}/meta.json"

    # Use jq -S for sorted comparison to avoid false negatives from key order differences
    lua_http_status=$(jq -S -c '.http_status' "${lua_metrics}" 2>/dev/null || echo "{}")
    meta_http_status=$(jq -S -c '.results.http_status' "${meta_json}" 2>/dev/null || echo "{}")

    if [[ "${lua_http_status}" != "${meta_http_status}" ]]; then
        log_fail "http_status mismatch"
        echo "    lua_metrics: ${lua_http_status}"
        echo "    meta.json:   ${meta_http_status}"
        return 1
    fi

    log_pass "meta.json http_status matches lua_metrics (${found_dir})"
}

test_raw_wrk_contains_done_output() {
    log_test "Checking raw_wrk.txt contains done handler output..."

    local results_dir raw_wrk_files
    results_dir=$(find_latest_results)
    raw_wrk_files=$(find "${results_dir}" -name "raw_wrk.txt" 2>/dev/null)

    if [[ -z "${results_dir}" || -z "${raw_wrk_files}" ]]; then
        log_fail "No raw_wrk.txt files found"
        return 1
    fi

    while IFS= read -r file; do
        if grep -q "HTTP Status Distribution" "${file}" 2>/dev/null; then
            log_pass "raw_wrk.txt contains done handler output"
            return 0
        fi
    done <<< "${raw_wrk_files}"

    log_fail "raw_wrk.txt does not contain 'HTTP Status Distribution'"
    return 1
}

test_error_rate_calculation() {
    log_test "Checking error_rate calculation..."

    local results_dir lua_metrics error_rate in_range
    results_dir=$(find_latest_results)

    if [[ -z "${results_dir}" ]]; then
        log_fail "No results directory found"
        return 1
    fi

    # Find lua_metrics.json: check both root and script-specific subdirectories
    if [[ -f "${results_dir}/lua_metrics.json" ]]; then
        lua_metrics="${results_dir}/lua_metrics.json"
    else
        lua_metrics=$(find "${results_dir}" -name "lua_metrics.json" -type f 2>/dev/null | head -1 || true)
    fi

    if [[ -z "${lua_metrics}" || ! -f "${lua_metrics}" ]]; then
        log_fail "lua_metrics.json not found"
        return 1
    fi

    error_rate=$(jq -r '.error_rate' "${lua_metrics}" 2>/dev/null || echo "null")
    in_range=$(awk -v rate="${error_rate}" 'BEGIN { print (rate >= 0 && rate <= 1) ? "yes" : "no" }')

    if [[ "${error_rate}" == "null" || "${in_range}" != "yes" ]]; then
        log_fail "error_rate is invalid: ${error_rate}"
        return 1
    fi

    log_pass "error_rate is valid: ${error_rate}"
}

test_phase_merging() {
    log_test "Checking phase merging..."

    local results_dir phase_dirs
    results_dir=$(find_latest_results)
    phase_dirs=$(find "${results_dir}" -maxdepth 1 -type d -name "phase_*" 2>/dev/null | wc -l)

    if [[ -z "${results_dir}" ]]; then
        log_fail "No results directory found"
        return 1
    fi

    if [[ "${phase_dirs}" -lt 2 ]]; then
        log_warn "Only ${phase_dirs} phase directory found, skipping phase merging test"
        return 0
    fi

    if [[ ! -f "${results_dir}/lua_metrics.json" ]]; then
        log_fail "Merged lua_metrics.json not found (expected with ${phase_dirs} phases)"
        return 1
    fi

    log_pass "Phase merging successful (${phase_dirs} phases merged)"
}

test_http_status_consistency() {
    log_test "Checking http_status sum equals total_requests..."

    local results_dir lua_metrics total_requests http_status_sum
    results_dir=$(find_latest_results)

    if [[ -z "${results_dir}" ]]; then
        log_fail "No results directory found"
        return 1
    fi

    # Find lua_metrics.json
    if [[ -f "${results_dir}/lua_metrics.json" ]]; then
        lua_metrics="${results_dir}/lua_metrics.json"
    else
        lua_metrics=$(find "${results_dir}" -name "lua_metrics.json" -type f 2>/dev/null | head -1 || true)
    fi

    if [[ -z "${lua_metrics}" || ! -f "${lua_metrics}" ]]; then
        log_fail "lua_metrics.json not found"
        return 1
    fi

    total_requests=$(jq -r '.total_requests // 0' "${lua_metrics}" 2>/dev/null)
    http_status_sum=$(jq -r '[.http_status | to_entries[] | .value] | add // 0' "${lua_metrics}" 2>/dev/null)

    if [[ "${total_requests}" -ne "${http_status_sum}" ]]; then
        log_fail "http_status sum (${http_status_sum}) != total_requests (${total_requests})"
        return 1
    fi

    log_pass "http_status sum (${http_status_sum}) equals total_requests (${total_requests})"
}

main() {
    parse_args "$@"

    echo ""
    echo "======================================"
    echo "HTTP Status Collection Pipeline Tests"
    echo "======================================"
    echo ""

    if ! command -v jq &> /dev/null; then
        echo -e "${RED}Error: jq is required but not installed${NC}"
        exit 1
    fi

    # Run benchmark if requested
    if [[ "${RUN_BENCHMARK}" == "true" ]]; then
        if ! run_quick_benchmark; then
            echo -e "${RED}Benchmark execution failed${NC}"
            exit 1
        fi
        echo ""
    fi

    local results_dir
    results_dir=$(find_latest_results)

    if [[ -z "${results_dir}" ]]; then
        echo -e "${RED}Error: No benchmark results found in ${BENCH_DIR}/results${NC}"
        echo "Please run a benchmark first:"
        echo "  cd ${BENCH_DIR}"
        echo "  ./run_benchmark.sh --scenario tasks_bulk --quick"
        echo ""
        echo "Or use --run flag to run benchmark automatically:"
        echo "  ./$(basename "$0") --run"
        exit 1
    fi

    echo "Testing results directory: ${results_dir}"
    echo ""

    test_lua_metrics_exists || true
    test_http_status_not_empty || true
    test_http_status_consistency || true
    test_meta_http_status_match || true
    test_raw_wrk_contains_done_output || true
    test_error_rate_calculation || true
    test_phase_merging || true

    echo ""
    echo "======================================"
    echo "Test Summary"
    echo "======================================"
    echo -e "${GREEN}Passed: ${TESTS_PASSED}${NC}"
    echo -e "${RED}Failed: ${TESTS_FAILED}${NC}"
    echo ""

    if [[ "${TESTS_FAILED}" -gt 0 ]]; then
        echo -e "${RED}Some tests failed${NC}"
        exit 1
    fi

    echo -e "${GREEN}All tests passed${NC}"
}

main "$@"
