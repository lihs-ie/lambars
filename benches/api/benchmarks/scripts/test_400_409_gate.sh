#!/usr/bin/env bash
# Test script for GATE-001: check_thresholds.sh 400 immediate FAIL

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_DIR="${SCRIPT_DIR}/.."
CHECK_THRESHOLDS="${BENCH_DIR}/check_thresholds.sh"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

PASSED=0
FAILED=0

# Test 1: tasks_update_steady with 400 > 0 should immediately FAIL (exit 3)
test_400_immediate_fail() {
    echo "Test 1: tasks_update_steady with 400 > 0 should immediately FAIL (exit 3)"

    local test_dir="/tmp/test_gate_001_fail"
    rm -rf "${test_dir}"
    mkdir -p "${test_dir}"

    cat > "${test_dir}/meta.json" <<'EOF'
{
  "results": {
    "requests": 1000,
    "http_status": {
      "200": 990,
      "400": 5,
      "409": 5
    },
    "p50": 20,
    "p90": 50,
    "p99": 100,
    "error_rate": 0.01,
    "conflict_rate": 0.005
  }
}
EOF

    # Run check_thresholds.sh (expecting exit 3)
    local exit_code=0
    "${CHECK_THRESHOLDS}" "${test_dir}" tasks_update_steady &>/dev/null || exit_code=$?

    if [[ ${exit_code} -eq 3 ]]; then
        echo -e "${GREEN}PASS${NC}: exit code is 3 (FAIL)"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}: exit code is ${exit_code}, expected 3"
        FAILED=$((FAILED + 1))
    fi

    rm -rf "${test_dir}"
}

# Test 2: tasks_update_steady with 400 == 0 and 409 > 0 should PASS threshold check
test_400_zero_pass() {
    echo "Test 2: tasks_update_steady with 400 == 0 and 409 > 0 should PASS 400 check"

    local test_dir="/tmp/test_gate_001_pass"
    rm -rf "${test_dir}"
    mkdir -p "${test_dir}"

    cat > "${test_dir}/meta.json" <<'EOF'
{
  "results": {
    "requests": 1000,
    "http_status": {
      "200": 995,
      "400": 0,
      "409": 5
    },
    "p50": 20,
    "p90": 50,
    "p99": 100,
    "error_rate": 0.005,
    "conflict_rate": 0.005
  }
}
EOF

    # Run check_thresholds.sh (expecting exit 0 or validation pass of 400 check)
    # Note: may fail on other thresholds, but should not fail on 400 check
    local exit_code=0
    local output
    output=$("${CHECK_THRESHOLDS}" "${test_dir}" tasks_update_steady 2>&1) || exit_code=$?

    # Check if output contains "Contract violation" message
    if echo "${output}" | grep -q "Contract violation"; then
        echo -e "${RED}FAIL${NC}: Found 'Contract violation' message, but 400 == 0"
        FAILED=$((FAILED + 1))
    else
        echo -e "${GREEN}PASS${NC}: No contract violation message (400 == 0)"
        PASSED=$((PASSED + 1))
    fi

    rm -rf "${test_dir}"
}

# Test 3: tasks_search_hot (non-tasks_update) should not trigger 400 check
test_non_tasks_update_scenario() {
    echo "Test 3: tasks_search_hot (non-tasks_update) should not trigger 400 check"

    local test_dir="/tmp/test_gate_001_search"
    rm -rf "${test_dir}"
    mkdir -p "${test_dir}"

    cat > "${test_dir}/meta.json" <<'EOF'
{
  "results": {
    "requests": 1000,
    "http_status": {
      "200": 995,
      "400": 5
    },
    "p50": 30,
    "p90": 100,
    "p99": 200,
    "error_rate": 0.005
  }
}
EOF

    # Run check_thresholds.sh with tasks_search_hot (expecting no 400 immediate FAIL)
    local exit_code=0
    local output
    output=$("${CHECK_THRESHOLDS}" "${test_dir}" tasks_search_hot 2>&1) || exit_code=$?

    # Check if output contains "Contract violation" message (should NOT appear)
    if echo "${output}" | grep -q "Contract violation"; then
        echo -e "${RED}FAIL${NC}: Found 'Contract violation' for non-tasks_update scenario"
        FAILED=$((FAILED + 1))
    else
        echo -e "${GREEN}PASS${NC}: No contract violation check for tasks_search_hot"
        PASSED=$((PASSED + 1))
    fi

    rm -rf "${test_dir}"
}

# Run all tests
echo "=== Running GATE-001 Tests ==="
echo ""
test_400_immediate_fail
echo ""
test_400_zero_pass
echo ""
test_non_tasks_update_scenario
echo ""
echo "=== Summary ==="
echo "Passed: ${PASSED}"
echo "Failed: ${FAILED}"
echo ""

if [[ ${FAILED} -gt 0 ]]; then
    echo -e "${RED}FAIL: ${FAILED} test(s) failed${NC}"
    exit 1
else
    echo -e "${GREEN}PASS: All tests passed${NC}"
    exit 0
fi
