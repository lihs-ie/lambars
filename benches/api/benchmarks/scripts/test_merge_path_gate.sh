#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../../.." && pwd)"
CHECK_THRESHOLDS="${PROJECT_ROOT}/benches/api/benchmarks/check_thresholds.sh"
TEMP_DIR="${PROJECT_ROOT}/benches/api/benchmarks/scripts/temp_test_merge_path_gate"

trap 'rm -rf "${TEMP_DIR}"' EXIT
mkdir -p "${TEMP_DIR}"

echo "Test 1: PASS case (with_arena_ratio = 0.95, tasks_bulk scenario)"
mkdir -p "${TEMP_DIR}/pass_case"
cat > "${TEMP_DIR}/pass_case/meta.json" <<'EOF'
{
  "results": {
    "http_status": {
      "200": 1000
    },
    "requests": 1000,
    "duration_seconds": 30,
    "latency_ms": {
      "p50": 5.0,
      "p90": 10.0,
      "p99": 20.0
    },
    "error_rate": 0.0,
    "merge_path_detail": {
      "with_arena_samples": 950,
      "without_arena_samples": 50,
      "with_arena_ratio": 0.95
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/pass_case" "tasks_bulk" > "${TEMP_DIR}/output_pass.txt" 2>&1; then
    echo "  PASS: Gate passed as expected"
    for pattern in "with_arena_ratio = 0.95" "PASS: Merge path telemetry within acceptable range"; do
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

echo ""
echo "Test 2: FAIL case (with_arena_ratio = 0.50, tasks_bulk scenario)"
mkdir -p "${TEMP_DIR}/fail_case"
cat > "${TEMP_DIR}/fail_case/meta.json" <<'EOF'
{
  "results": {
    "http_status": {
      "200": 1000
    },
    "requests": 1000,
    "duration_seconds": 30,
    "latency_ms": {
      "p50": 5.0,
      "p90": 10.0,
      "p99": 20.0
    },
    "error_rate": 0.0,
    "merge_path_detail": {
      "with_arena_samples": 500,
      "without_arena_samples": 500,
      "with_arena_ratio": 0.50
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/fail_case" "tasks_bulk" > "${TEMP_DIR}/output_fail.txt" 2>&1; then
    echo "  FAIL: Gate should have failed when with_arena_ratio < 0.90"
    cat "${TEMP_DIR}/output_fail.txt"
    exit 1
else
    EXIT_CODE=$?
    if [[ ${EXIT_CODE} -ne 3 ]]; then
        echo "  FAIL: Gate failed with unexpected exit code ${EXIT_CODE} (expected 3)"
        cat "${TEMP_DIR}/output_fail.txt"
        exit 1
    fi

    echo "  PASS: Gate failed with exit 3 as expected"
    for pattern in "FAIL: Merge path regression detected" \
                   "with_arena_ratio = 0.50 (must be >= 0.90)"; do
        if ! grep -q "$pattern" "${TEMP_DIR}/output_fail.txt"; then
            echo "  ERROR: Expected pattern not found: $pattern"
            cat "${TEMP_DIR}/output_fail.txt"
            exit 1
        fi
    done
fi

echo ""
echo "Test 3: WARNING case (merge_path_detail missing, tasks_bulk scenario)"
mkdir -p "${TEMP_DIR}/warning_case"
cat > "${TEMP_DIR}/warning_case/meta.json" <<'EOF'
{
  "results": {
    "http_status": {
      "200": 1000
    },
    "requests": 1000,
    "duration_seconds": 30,
    "latency_ms": {
      "p50": 5.0,
      "p90": 10.0,
      "p99": 20.0
    },
    "error_rate": 0.0
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/warning_case" "tasks_bulk" > "${TEMP_DIR}/output_warning.txt" 2>&1; then
    echo "  PASS: Gate passed with warning as expected (merge_path_detail missing)"
    if ! grep -q "WARNING: .results.merge_path_detail not found in meta.json" "${TEMP_DIR}/output_warning.txt"; then
        echo "  ERROR: Expected warning not found"
        cat "${TEMP_DIR}/output_warning.txt"
        exit 1
    fi
else
    echo "  FAIL: Gate should have passed with warning when merge_path_detail is missing"
    cat "${TEMP_DIR}/output_warning.txt"
    exit 1
fi

echo ""
echo "Test 4: SKIP case (tasks_update scenario does not check merge_path)"
mkdir -p "${TEMP_DIR}/skip_case"
cat > "${TEMP_DIR}/skip_case/meta.json" <<'EOF'
{
  "results": {
    "http_status": {
      "200": 1000,
      "400": 0
    },
    "requests": 1000,
    "duration_seconds": 30,
    "latency_ms": {
      "p50": 5.0,
      "p90": 10.0,
      "p99": 20.0
    },
    "error_rate": 0.0,
    "conflict_rate": 0.0,
    "merge_path_detail": {
      "with_arena_samples": 100,
      "without_arena_samples": 900,
      "with_arena_ratio": 0.10
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/skip_case" "tasks_update" > "${TEMP_DIR}/output_skip.txt" 2>&1; then
    echo "  PASS: Gate passed (merge_path check skipped for tasks_update)"
    if grep -q "Merge path telemetry" "${TEMP_DIR}/output_skip.txt"; then
        echo "  ERROR: Merge path check should be skipped for tasks_update"
        cat "${TEMP_DIR}/output_skip.txt"
        exit 1
    fi
else
    echo "  FAIL: Gate should have passed for tasks_update (merge_path check should be skipped)"
    cat "${TEMP_DIR}/output_skip.txt"
    exit 1
fi

echo ""
echo "All tests passed!"
