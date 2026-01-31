#!/bin/bash
# Test suite for shell scripts

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCHMARKS_DIR="$(dirname "${SCRIPT_DIR}")"

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

assert_equals() {
    local expected="$1" actual="$2" test_name="$3"
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

assert_contains() {
    local pattern="$1" text="$2" test_name="$3"
    ((TESTS_RUN++))
    if echo "${text}" | grep -q "${pattern}"; then
        log_pass "${test_name}"
        ((TESTS_PASSED++))
    else
        log_fail "${test_name}"
        echo "  Pattern: ${pattern}"
        echo "  Text:    ${text}"
        ((TESTS_FAILED++))
        return 1
    fi
}

assert_file_exists() {
    local file="$1" test_name="$2"
    ((TESTS_RUN++))
    if [[ -f "${file}" ]]; then
        log_pass "${test_name}"
        ((TESTS_PASSED++))
    else
        log_fail "${test_name}"
        echo "  File not found: ${file}"
        ((TESTS_FAILED++))
        return 1
    fi
}

test_scenario_env() {
    cat <<EOF

==============================================
  Testing scenario_env.sh
==============================================

EOF
    local test_scenario
    test_scenario=$(mktemp) || { log_fail "Failed to create temp file"; return 1; }

    cat > "${test_scenario}" << 'EOF'
name: test_scenario
storage_mode: postgres
cache_mode: redis
data_size: medium
hit_rate: 75
cache_strategy: write-through
load_pattern: burst
threads: 4
connections: 20
duration_seconds: 60
endpoint: /tasks
payload: large
database_pool_size: 32
redis_pool_size: 16
worker_threads: 8
retry: true
fail_rate: 0.05
EOF

    # shellcheck source=scenario_env.sh
    source "${SCRIPT_DIR}/scenario_env.sh"

    log_test "Loading scenario environment variables"
    ((TESTS_RUN++))
    if load_scenario_env "${test_scenario}"; then
        log_pass "load_scenario_env executed successfully"
        ((TESTS_PASSED++))
    else
        log_fail "load_scenario_env failed"
        ((TESTS_FAILED++))
    fi
    assert_equals "test_scenario" "${_SCENARIO_NAME}" "SCENARIO_NAME parsing"
    assert_equals "postgres" "${_STORAGE_MODE}" "STORAGE_MODE parsing"
    assert_equals "redis" "${_CACHE_MODE}" "CACHE_MODE parsing"
    assert_equals "1e4" "${_DATA_SCALE}" "DATA_SCALE mapping (medium -> 1e4)"
    assert_equals "75" "${_HIT_RATE}" "HIT_RATE parsing"
    assert_equals "write-through" "${_CACHE_STRATEGY}" "CACHE_STRATEGY parsing"
    assert_equals "burst" "${_RPS_PROFILE}" "RPS_PROFILE mapping"
    assert_equals "4" "${_THREADS}" "THREADS parsing"
    assert_equals "20" "${_CONNECTIONS}" "CONNECTIONS parsing"
    assert_equals "60" "${_DURATION}" "DURATION parsing"
    assert_equals "/tasks" "${_ENDPOINT}" "ENDPOINT parsing"
    assert_equals "large" "${_PAYLOAD}" "PAYLOAD mapping"

    # Cleanup
    rm -f "${test_scenario}"
}

test_scenario_env_req_bench_002() {
    cat <<EOF

==============================================
  Testing scenario_env.sh (REQ-BENCH-002)
==============================================

EOF
    local test_scenario
    test_scenario=$(mktemp) || { log_fail "Failed to create temp file"; return 1; }

    cat > "${test_scenario}" << 'EOF'
name: tasks_search_hot
description: "GET /tasks/search hot cache"
storage: postgres
cache: redis
data_scale: 1000000
payload: medium
rps_profile: steady
hit_rate: 90
script: tasks_search
endpoint: "/tasks/search"
threads: 4
connections: 20
duration_seconds: 60
EOF

    # shellcheck source=scenario_env.sh
    source "${SCRIPT_DIR}/scenario_env.sh"

    log_test "Loading REQ-BENCH-002 compliant scenario"
    ((TESTS_RUN++))
    if load_scenario_env "${test_scenario}"; then
        log_pass "load_scenario_env executed successfully"
        ((TESTS_PASSED++))
    else
        log_fail "load_scenario_env failed"
        ((TESTS_FAILED++))
        rm -f "${test_scenario}"
        return
    fi
    assert_equals "tasks_search_hot" "${_SCENARIO_NAME}" "SCENARIO_NAME (REQ-BENCH-002)"
    assert_equals "postgres" "${_STORAGE_MODE}" "STORAGE_MODE (storage key)"
    assert_equals "redis" "${_CACHE_MODE}" "CACHE_MODE (cache key)"
    assert_equals "1000000" "${_DATA_SCALE}" "DATA_SCALE (direct value)"
    assert_equals "90" "${_HIT_RATE}" "HIT_RATE"
    assert_equals "steady" "${_RPS_PROFILE}" "RPS_PROFILE (direct value)"
    assert_equals "4" "${_THREADS}" "THREADS"
    assert_equals "20" "${_CONNECTIONS}" "CONNECTIONS"
    assert_equals "60" "${_DURATION}" "DURATION"
    assert_equals "/tasks/search" "${_ENDPOINT}" "ENDPOINT"
    assert_equals "medium" "${_PAYLOAD}" "PAYLOAD"

    # Cleanup
    rm -f "${test_scenario}"
}

test_compare_commits() {
    cat <<EOF

==============================================
  Testing compare_commits.sh
==============================================

EOF
    log_test "Testing --help option"
    assert_contains "Usage:" "$("${SCRIPT_DIR}/compare_commits.sh" --help 2>&1 || true)" "compare_commits.sh --help displays usage"

    log_test "Testing missing arguments"
    assert_contains "Missing required arguments" "$("${SCRIPT_DIR}/compare_commits.sh" 2>&1 || true)" "compare_commits.sh validates required arguments"
}

test_extract_hot_functions() {
    cat <<EOF

==============================================
  Testing extract_hot_functions.sh
==============================================

EOF
    log_test "Testing --help option"
    assert_contains "Usage:" "$("${SCRIPT_DIR}/extract_hot_functions.sh" --help 2>&1 || true)" "extract_hot_functions.sh --help displays usage"

    log_test "Testing missing arguments"
    assert_contains "Missing perf data file" "$("${SCRIPT_DIR}/extract_hot_functions.sh" 2>&1 || true)" "extract_hot_functions.sh validates required arguments"
}

test_compare_results_json() {
    cat <<EOF

==============================================
  Testing compare_results.sh JSON parsing
==============================================

EOF
    local test_json_dir
    test_json_dir=$(mktemp -d) || { log_fail "Failed to create temp directory"; return 1; }

    mkdir -p "${test_json_dir}/base"
    mkdir -p "${test_json_dir}/new"

    cat > "${test_json_dir}/base/wrk-output.json" << 'EOF'
{
  "scenario": {
    "name": "test_scenario",
    "storage_mode": "in_memory",
    "cache_mode": "none"
  },
  "execution": {
    "timestamp": "2026-01-31T12:00:00Z",
    "duration_seconds": 30,
    "threads": 2,
    "connections": 10
  },
  "latency": {
    "mean": "5.23ms",
    "percentiles": {
      "p50": "4.12ms",
      "p75": "6.34ms",
      "p90": "8.56ms",
      "p99": "15.23ms"
    }
  },
  "throughput": {
    "requests_total": 10000,
    "requests_per_second": 333.33
  },
  "errors": {
    "connect": 0,
    "read": 0,
    "write": 0,
    "timeout": 0
  }
}
EOF

    cat > "${test_json_dir}/new/wrk-output.json" << 'EOF'
{
  "scenario": {
    "name": "test_scenario",
    "storage_mode": "in_memory",
    "cache_mode": "none"
  },
  "execution": {
    "timestamp": "2026-01-31T12:30:00Z",
    "duration_seconds": 30,
    "threads": 2,
    "connections": 10
  },
  "latency": {
    "mean": "4.50ms",
    "percentiles": {
      "p50": "3.80ms",
      "p75": "5.50ms",
      "p90": "7.20ms",
      "p99": "12.50ms"
    }
  },
  "throughput": {
    "requests_total": 12000,
    "requests_per_second": 400.00
  },
  "errors": {
    "connect": 0,
    "read": 0,
    "write": 0,
    "timeout": 0
  }
}
EOF

    log_test "Testing compare_results.sh with wrk-output.json"
    local compare_output
    compare_output=$("${BENCHMARKS_DIR}/compare_results.sh" "${test_json_dir}/base" "${test_json_dir}/new" 2>&1 || true)

    assert_contains "p50" "${compare_output}" "compare_results.sh displays p50 latency"
    assert_contains "p90" "${compare_output}" "compare_results.sh displays p90 latency"
    assert_contains "p99" "${compare_output}" "compare_results.sh displays p99 latency"
    assert_contains "rps" "${compare_output}" "compare_results.sh displays RPS"

    # Cleanup
    rm -rf "${test_json_dir}"
}

test_file_existence() {
    cat <<EOF

==============================================
  Testing file existence
==============================================

EOF
    assert_file_exists "${SCRIPT_DIR}/scenario_env.sh" "scenario_env.sh exists"
    assert_file_exists "${SCRIPT_DIR}/compare_commits.sh" "compare_commits.sh exists"
    assert_file_exists "${SCRIPT_DIR}/extract_hot_functions.sh" "extract_hot_functions.sh exists"
    assert_file_exists "${BENCHMARKS_DIR}/compare_results.sh" "compare_results.sh exists"
}

main() {
    cat <<EOF

==============================================
  Shell Script Test Suite
==============================================

EOF

    test_file_existence
    test_scenario_env
    test_scenario_env_req_bench_002
    test_compare_commits
    test_extract_hot_functions
    test_compare_results_json

    cat <<EOF

==============================================
  Test Summary
==============================================

Tests run:    ${TESTS_RUN}
Tests passed: ${GREEN}${TESTS_PASSED}${NC}
Tests failed: ${RED}${TESTS_FAILED}${NC}

EOF

    if [[ ${TESTS_FAILED} -eq 0 ]]; then
        echo -e "${GREEN}${BOLD}All tests passed!${NC}"
        exit 0
    else
        echo -e "${RED}${BOLD}Some tests failed.${NC}"
        exit 1
    fi
}

main "$@"
