#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../../.." && pwd)"
CHECK_THRESHOLDS="${PROJECT_ROOT}/benches/api/benchmarks/check_thresholds.sh"
TEMP_DIR="${PROJECT_ROOT}/benches/api/benchmarks/scripts/temp_test_merge_path_gate"

trap 'rm -rf "${TEMP_DIR}"' EXIT
mkdir -p "${TEMP_DIR}"

echo "Test 1: PASS case (bulk_with_arena_ratio = 0.95, tasks_bulk scenario)"
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
      "bulk_with_arena": 950,
      "bulk_without_arena": 50,
      "bulk_with_arena_ratio": 0.95
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/pass_case" "tasks_bulk" > "${TEMP_DIR}/output_pass.txt" 2>&1; then
    echo "  PASS: Gate passed as expected"
    for pattern in "bulk_with_arena_ratio = 0.950000" "PASS: Merge path telemetry within acceptable range"; do
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
echo "Test 2: FAIL case (bulk_with_arena_ratio = 0.50, tasks_bulk scenario)"
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
      "bulk_with_arena": 500,
      "bulk_without_arena": 500,
      "bulk_with_arena_ratio": 0.50
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/fail_case" "tasks_bulk" > "${TEMP_DIR}/output_fail.txt" 2>&1; then
    echo "  FAIL: Gate should have failed when bulk_with_arena_ratio < 0.90"
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
                   "bulk_with_arena_ratio = 0.500000 (must be >= 0.90)"; do
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
      "bulk_with_arena": 100,
      "bulk_without_arena": 900,
      "bulk_with_arena_ratio": 0.10
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
echo "Test 5: Boundary case (bulk_with_arena_ratio = 0.90 exactly, should PASS)"
mkdir -p "${TEMP_DIR}/boundary_case"
cat > "${TEMP_DIR}/boundary_case/meta.json" <<'EOF'
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
      "bulk_with_arena": 900,
      "bulk_without_arena": 100,
      "bulk_with_arena_ratio": 0.90
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/boundary_case" "tasks_bulk" > "${TEMP_DIR}/output_boundary.txt" 2>&1; then
    echo "  PASS: Gate passed at boundary (ratio = 0.90)"
    if ! grep -q "bulk_with_arena_ratio = 0.900000" "${TEMP_DIR}/output_boundary.txt"; then
        echo "  ERROR: Expected ratio not found in output"
        cat "${TEMP_DIR}/output_boundary.txt"
        exit 1
    fi
else
    echo "  FAIL: Gate should pass when ratio equals threshold"
    cat "${TEMP_DIR}/output_boundary.txt"
    exit 1
fi

echo ""
echo "Test 6: Ratio mismatch case (stored ratio differs from calculated, should use calculated)"
mkdir -p "${TEMP_DIR}/mismatch_case"
cat > "${TEMP_DIR}/mismatch_case/meta.json" <<'EOF'
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
      "bulk_with_arena": 920,
      "bulk_without_arena": 80,
      "bulk_with_arena_ratio": 0.50
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/mismatch_case" "tasks_bulk" > "${TEMP_DIR}/output_mismatch.txt" 2>&1; then
    echo "  PASS: Gate passed using recalculated ratio (ignoring stored 0.50)"
    # Calculated ratio should be 920/(920+80) = 0.92
    if ! grep -q "bulk_with_arena_ratio = 0.920000" "${TEMP_DIR}/output_mismatch.txt"; then
        echo "  ERROR: Expected recalculated ratio 0.920000 not found"
        cat "${TEMP_DIR}/output_mismatch.txt"
        exit 1
    fi
else
    echo "  FAIL: Gate should pass with recalculated ratio 0.92 (not stored 0.50)"
    cat "${TEMP_DIR}/output_mismatch.txt"
    exit 1
fi

echo ""
echo "All tests passed!"
