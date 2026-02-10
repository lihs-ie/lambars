#!/usr/bin/env bash
# Test script for GATE-001 Extended: check_thresholds.sh 400 gate should exclude PATCH scenarios

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

# Test 1: tasks_update_status (PATCH) with 400 > 0 should NOT trigger 400 fail-fast gate
test_400_gate_patch_exclusion() {
    echo "Test 1: tasks_update_status (PATCH) with 400 > 0 should NOT trigger 400 fail-fast"

    local test_dir="/tmp/test_gate_patch_exclusion"
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

    # Run check_thresholds.sh with tasks_update_status (expecting no 400 immediate FAIL)
    local exit_code=0
    local output
    output=$("${CHECK_THRESHOLDS}" "${test_dir}" tasks_update_status 2>&1) || exit_code=$?

    # Check if output contains "Contract violation" message (should NOT appear for PATCH)
    if echo "${output}" | grep -q "Contract violation"; then
        echo -e "${RED}FAIL${NC}: Found 'Contract violation' for PATCH scenario (tasks_update_status)"
        echo "Output: ${output}"
        FAILED=$((FAILED + 1))
    else
        echo -e "${GREEN}PASS${NC}: No 400 fail-fast gate for tasks_update_status (PATCH)"
        PASSED=$((PASSED + 1))
    fi

    rm -rf "${test_dir}"
}

# Test 2: tasks_update (PUT) with 400 > 0 should trigger 400 fail-fast gate (baseline)
test_400_gate_put_baseline() {
    echo "Test 2: tasks_update (PUT) with 400 > 0 should trigger 400 fail-fast (baseline)"

    local test_dir="/tmp/test_gate_put_baseline"
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
    "error_rate": 0.0005,
    "conflict_rate": 0.005
  }
}
EOF

    # Run check_thresholds.sh with tasks_update_steady (expecting exit 3)
    local exit_code=0
    "${CHECK_THRESHOLDS}" "${test_dir}" tasks_update_steady &>/dev/null || exit_code=$?

    if [[ ${exit_code} -eq 3 ]]; then
        echo -e "${GREEN}PASS${NC}: tasks_update_steady triggers 400 fail-fast (exit 3)"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}: exit code is ${exit_code}, expected 3"
        FAILED=$((FAILED + 1))
    fi

    rm -rf "${test_dir}"
}

# Run all tests
echo "=== Running GATE-001 Extended Tests (PATCH scenario exclusion) ==="
echo ""
test_400_gate_patch_exclusion
echo ""
test_400_gate_put_baseline
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
