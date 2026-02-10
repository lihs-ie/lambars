#!/usr/bin/env bash
# Test script for GATE-002 Extended: validate_metrics_invariants.sh Invariant 4 with benchmark/meta path

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_DIR="${SCRIPT_DIR}/.."
VALIDATE_INVARIANTS="${BENCH_DIR}/validate_metrics_invariants.sh"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

PASSED=0
FAILED=0

# Test 1: tasks_update_steady with benchmark/meta/*.json path and 400 > 0 should violate Invariant 4
test_invariant4_benchmark_meta_path_violation() {
    echo "Test 1: tasks_update_steady with benchmark/meta/*.json path and 400 > 0 should violate Invariant 4"

    local test_dir="/tmp/test_invariant4_benchmark_meta_violation/tasks_update_steady/benchmark/meta"
    rm -rf "/tmp/test_invariant4_benchmark_meta_violation"
    mkdir -p "${test_dir}"

    cat > "${test_dir}/tasks_update_steady.json" <<'EOF'
{
  "results": {
    "requests": 1000,
    "http_status": {
      "200": 995,
      "400": 5
    },
    "latency_ms": {
      "p50": 20,
      "p99": 100
    },
    "error_rate": 0.005
  },
  "errors": {
    "http_4xx": 5,
    "http_5xx": 0,
    "socket_errors": {
      "total": 0
    }
  }
}
EOF

    # Run validate_metrics_invariants.sh (expecting failure with Invariant 4 violation)
    local exit_code=0
    local output
    output=$("${VALIDATE_INVARIANTS}" --all /tmp/test_invariant4_benchmark_meta_violation 2>&1) || exit_code=$?

    # Check if output contains "PUT contract violation" message
    if echo "${output}" | grep -q "PUT contract violation"; then
        echo -e "${GREEN}PASS${NC}: Found 'PUT contract violation' message"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}: Missing 'PUT contract violation' message"
        echo "Output: ${output}"
        FAILED=$((FAILED + 1))
    fi

    # Check exit code (should be 1 for violations)
    if [[ ${exit_code} -eq 1 ]]; then
        echo -e "${GREEN}PASS${NC}: exit code is 1 (violation detected)"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}: exit code is ${exit_code}, expected 1"
        FAILED=$((FAILED + 1))
    fi

    rm -rf "/tmp/test_invariant4_benchmark_meta_violation"
}

# Test 2: tasks_update_status with benchmark/meta/*.json path and 400 > 0 should NOT violate Invariant 4 (PATCH scenario)
test_invariant4_benchmark_meta_path_patch_exclusion() {
    echo "Test 2: tasks_update_status with benchmark/meta/*.json path and 400 > 0 should NOT violate Invariant 4"

    local test_dir="/tmp/test_invariant4_benchmark_meta_patch/tasks_update_status/benchmark/meta"
    rm -rf "/tmp/test_invariant4_benchmark_meta_patch"
    mkdir -p "${test_dir}"

    cat > "${test_dir}/tasks_update_status.json" <<'EOF'
{
  "results": {
    "requests": 1000,
    "http_status": {
      "200": 995,
      "400": 5
    },
    "latency_ms": {
      "p50": 20,
      "p99": 100
    },
    "error_rate": 0.005
  },
  "errors": {
    "http_4xx": 5,
    "http_5xx": 0,
    "socket_errors": {
      "total": 0
    }
  }
}
EOF

    # Run validate_metrics_invariants.sh (expecting success - no Invariant 4 check for PATCH)
    local exit_code=0
    local output
    output=$("${VALIDATE_INVARIANTS}" --all /tmp/test_invariant4_benchmark_meta_patch 2>&1) || exit_code=$?

    # Check if output does NOT contain "PUT contract violation" message
    if echo "${output}" | grep -q "PUT contract violation"; then
        echo -e "${RED}FAIL${NC}: Found 'PUT contract violation' for PATCH scenario"
        echo "Output: ${output}"
        FAILED=$((FAILED + 1))
    else
        echo -e "${GREEN}PASS${NC}: No Invariant 4 check for tasks_update_status (PATCH)"
        PASSED=$((PASSED + 1))
    fi

    # Check exit code (should be 0 for all pass)
    if [[ ${exit_code} -eq 0 ]]; then
        echo -e "${GREEN}PASS${NC}: exit code is 0 (all pass)"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}: exit code is ${exit_code}, expected 0"
        FAILED=$((FAILED + 1))
    fi

    rm -rf "/tmp/test_invariant4_benchmark_meta_patch"
}

# Run all tests
echo "=== Running GATE-002 Extended Tests (Invariant 4 with benchmark/meta path) ==="
echo ""
test_invariant4_benchmark_meta_path_violation
echo ""
test_invariant4_benchmark_meta_path_patch_exclusion
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
