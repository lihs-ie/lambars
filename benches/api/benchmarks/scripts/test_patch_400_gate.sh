#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../../.." && pwd)"
CHECK_THRESHOLDS="${PROJECT_ROOT}/benches/api/benchmarks/check_thresholds.sh"
TEMP_DIR="${PROJECT_ROOT}/benches/api/benchmarks/scripts/temp_test_patch_400_gate"

trap 'rm -rf "${TEMP_DIR}"' EXIT
mkdir -p "${TEMP_DIR}"

echo "Test 1: PASS case (400 = 0)"
mkdir -p "${TEMP_DIR}/pass_case"
cat > "${TEMP_DIR}/pass_case/meta.json" <<'EOF'
{
  "results": {
    "http_status": {
      "200": 1000,
      "409": 10
    },
    "requests": 1010,
    "duration_seconds": 30,
    "latency_ms": {
      "p50": 10.5,
      "p90": 15.2,
      "p99": 25.8
    },
    "error_rate": 0.0,
    "conflict_rate": 0.0
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/pass_case" "tasks_update_status" > "${TEMP_DIR}/output_pass.txt" 2>&1; then
    echo "  PASS: Gate passed as expected"
    for pattern in "validation_error_rate (400) = 0.000000" "conflict_error_rate (409) = 0.009901"; do
        if ! grep -q "$pattern" "${TEMP_DIR}/output_pass.txt"; then
            echo "  ERROR: Expected pattern not found: $pattern"
            cat "${TEMP_DIR}/output_pass.txt"
            exit 1
        fi
    done
else
    echo "  FAIL: Gate should have passed"
    cat "${TEMP_DIR}/output_pass.txt"
    exit 1
fi

# Test 2: FAIL case (400 > 0)
echo ""
echo "Test 2: FAIL case (400 > 0)"
mkdir -p "${TEMP_DIR}/fail_case"
cat > "${TEMP_DIR}/fail_case/meta.json" <<'EOF'
{
  "results": {
    "http_status": {
      "200": 990,
      "400": 5,
      "409": 15
    },
    "requests": 1010,
    "duration_seconds": 30,
    "latency_ms": {
      "p50": 10.5,
      "p90": 15.2,
      "p99": 25.8
    },
    "error_rate": 0.0,
    "conflict_rate": 0.0
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/fail_case" "tasks_update_status" > "${TEMP_DIR}/output_fail.txt" 2>&1; then
    echo "  FAIL: Gate should have failed when 400 > 0"
    cat "${TEMP_DIR}/output_fail.txt"
    exit 1
else
    EXIT_CODE=$?
    if [[ ${EXIT_CODE} -ne 3 ]]; then
        echo "  FAIL: Gate failed with unexpected exit code ${EXIT_CODE}"
        cat "${TEMP_DIR}/output_fail.txt"
        exit 1
    fi

    echo "  PASS: Gate failed with exit 3 as expected"
    for pattern in "FAIL: Transition validation error - invalid status transition in PATCH payload" \
                   "http_status.400 = 5 (must be 0)"; do
        if ! grep -q "$pattern" "${TEMP_DIR}/output_fail.txt"; then
            echo "  ERROR: Expected pattern not found: $pattern"
            cat "${TEMP_DIR}/output_fail.txt"
            exit 1
        fi
    done
fi

echo ""
echo "All tests passed!"
