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
    "rps": 400,
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
    "rps": 400,
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
echo "Test 3: FAIL case (merge_path_detail missing, tasks_bulk scenario)"
mkdir -p "${TEMP_DIR}/fail_missing_case"
cat > "${TEMP_DIR}/fail_missing_case/meta.json" <<'EOF'
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
    "rps": 400
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/fail_missing_case" "tasks_bulk" > "${TEMP_DIR}/output_fail_missing.txt" 2>&1; then
    echo "  FAIL: Gate should have failed when merge_path_detail is missing"
    cat "${TEMP_DIR}/output_fail_missing.txt"
    exit 1
else
    EXIT_CODE=$?
    if [[ ${EXIT_CODE} -ne 3 ]]; then
        echo "  FAIL: Gate failed with unexpected exit code ${EXIT_CODE} (expected 3)"
        cat "${TEMP_DIR}/output_fail_missing.txt"
        exit 1
    fi

    echo "  PASS: Gate failed with exit 3 as expected (merge_path_detail missing)"
    if ! grep -q "FAIL.*merge_path_detail not found" "${TEMP_DIR}/output_fail_missing.txt"; then
        echo "  ERROR: Expected FAIL message not found"
        cat "${TEMP_DIR}/output_fail_missing.txt"
        exit 1
    fi
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
    "rps": 400,
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
    "rps": 400,
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
echo "Test 7: Reverse mismatch case (stored ratio high but recalculated is low, should FAIL)"
mkdir -p "${TEMP_DIR}/reverse_mismatch_case"
cat > "${TEMP_DIR}/reverse_mismatch_case/meta.json" <<'EOF'
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
    "rps": 400,
    "merge_path_detail": {
      "bulk_with_arena": 100,
      "bulk_without_arena": 900,
      "bulk_with_arena_ratio": 0.99
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/reverse_mismatch_case" "tasks_bulk" > "${TEMP_DIR}/output_reverse_mismatch.txt" 2>&1; then
    echo "  FAIL: Gate should have failed when recalculated ratio is low (0.10) despite stored 0.99"
    cat "${TEMP_DIR}/output_reverse_mismatch.txt"
    exit 1
else
    EXIT_CODE=$?
    if [[ ${EXIT_CODE} -ne 3 ]]; then
        echo "  FAIL: Gate failed with unexpected exit code ${EXIT_CODE} (expected 3)"
        cat "${TEMP_DIR}/output_reverse_mismatch.txt"
        exit 1
    fi

    echo "  PASS: Gate failed with exit 3 as expected (recalculated ratio 0.10 < 0.90)"
    # Calculated ratio should be 100/(100+900) = 0.10
    if ! grep -q "bulk_with_arena_ratio = 0.100000 (must be >= 0.90)" "${TEMP_DIR}/output_reverse_mismatch.txt"; then
        echo "  ERROR: Expected recalculated ratio 0.100000 not found"
        cat "${TEMP_DIR}/output_reverse_mismatch.txt"
        exit 1
    fi
fi

echo ""
echo "Test 8: Invalid numeric fields should exit 2"
mkdir -p "${TEMP_DIR}/invalid_numeric_case"
cat > "${TEMP_DIR}/invalid_numeric_case/meta.json" <<'EOF'
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
    "rps": 400,
    "merge_path_detail": {
      "bulk_with_arena": "12x",
      "bulk_without_arena": 10,
      "bulk_with_arena_ratio": 0.50
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/invalid_numeric_case" "tasks_bulk" > "${TEMP_DIR}/output_invalid_numeric.txt" 2>&1; then
    echo "  FAIL: Gate should have failed with exit 2 for invalid numeric fields"
    cat "${TEMP_DIR}/output_invalid_numeric.txt"
    exit 1
else
    EXIT_CODE=$?
    if [[ ${EXIT_CODE} -ne 2 ]]; then
        echo "  FAIL: Gate failed with unexpected exit code ${EXIT_CODE} (expected 2)"
        cat "${TEMP_DIR}/output_invalid_numeric.txt"
        exit 1
    fi

    echo "  PASS: Gate failed with exit 2 as expected (invalid numeric fields)"
    if ! grep -q "merge_path_detail.bulk_\\* must be non-negative integers" "${TEMP_DIR}/output_invalid_numeric.txt"; then
        echo "  ERROR: Expected error message not found"
        cat "${TEMP_DIR}/output_invalid_numeric.txt"
        exit 1
    fi
fi

echo ""
echo "Test 9: FAIL case (bulk_with_arena and bulk_without_arena are empty strings)"
mkdir -p "${TEMP_DIR}/empty_fields_case"
cat > "${TEMP_DIR}/empty_fields_case/meta.json" <<'EOF'
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
    "rps": 400,
    "merge_path_detail": {
      "bulk_with_arena": "",
      "bulk_without_arena": ""
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/empty_fields_case" "tasks_bulk" > "${TEMP_DIR}/output_empty_fields.txt" 2>&1; then
    echo "  FAIL: Gate should have failed when bulk_with_arena/bulk_without_arena are empty strings"
    cat "${TEMP_DIR}/output_empty_fields.txt"
    exit 1
else
    EXIT_CODE=$?
    if [[ ${EXIT_CODE} -ne 3 ]]; then
        echo "  FAIL: Gate failed with unexpected exit code ${EXIT_CODE} (expected 3)"
        cat "${TEMP_DIR}/output_empty_fields.txt"
        exit 1
    fi

    echo "  PASS: Gate failed with exit 3 as expected (empty fields)"
    if ! grep -q "FAIL.*merge_path_detail fields incomplete" "${TEMP_DIR}/output_empty_fields.txt"; then
        echo "  ERROR: Expected FAIL message not found"
        cat "${TEMP_DIR}/output_empty_fields.txt"
        exit 1
    fi
fi

echo ""
echo "Test 10: FAIL case (merge_path_error field only, no bulk fields)"
mkdir -p "${TEMP_DIR}/merge_path_error_case"
cat > "${TEMP_DIR}/merge_path_error_case/meta.json" <<'EOF'
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
    "rps": 400,
    "merge_path_detail": {
      "merge_path_error": "stacks.folded not found"
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/merge_path_error_case" "tasks_bulk" > "${TEMP_DIR}/output_merge_path_error.txt" 2>&1; then
    echo "  FAIL: Gate should have failed when only merge_path_error exists"
    cat "${TEMP_DIR}/output_merge_path_error.txt"
    exit 1
else
    EXIT_CODE=$?
    if [[ ${EXIT_CODE} -ne 3 ]]; then
        echo "  FAIL: Gate failed with unexpected exit code ${EXIT_CODE} (expected 3)"
        cat "${TEMP_DIR}/output_merge_path_error.txt"
        exit 1
    fi

    echo "  PASS: Gate failed with exit 3 as expected (merge_path_error only)"
    if ! grep -q "FAIL.*merge_path_detail fields incomplete" "${TEMP_DIR}/output_merge_path_error.txt"; then
        echo "  ERROR: Expected FAIL message not found"
        cat "${TEMP_DIR}/output_merge_path_error.txt"
        exit 1
    fi
fi

echo ""
echo "Test 11: PASS case (regression guard passes with p99=8000ms, rps=400)"
mkdir -p "${TEMP_DIR}/regression_pass_case"
cat > "${TEMP_DIR}/regression_pass_case/meta.json" <<'EOF'
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
      "p99": 8000.0
    },
    "error_rate": 0.0,
    "rps": 400,
    "merge_path_detail": {
      "bulk_with_arena": 950,
      "bulk_without_arena": 50,
      "bulk_with_arena_ratio": 0.95
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/regression_pass_case" "tasks_bulk" > "${TEMP_DIR}/output_regression_pass.txt" 2>&1; then
    echo "  PASS: Gate passed (p99=8000ms <= 9550ms, rps=400 >= 341.36)"
else
    echo "  FAIL: Gate should have passed when regression guard thresholds are met"
    cat "${TEMP_DIR}/output_regression_pass.txt"
    exit 1
fi

echo ""
echo "Test 12: FAIL case (regression guard fails with p99=10000ms exceeding revert)"
mkdir -p "${TEMP_DIR}/regression_fail_p99_case"
cat > "${TEMP_DIR}/regression_fail_p99_case/meta.json" <<'EOF'
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
      "p99": 10000.0
    },
    "error_rate": 0.0,
    "rps": 400,
    "merge_path_detail": {
      "bulk_with_arena": 950,
      "bulk_without_arena": 50,
      "bulk_with_arena_ratio": 0.95
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/regression_fail_p99_case" "tasks_bulk" > "${TEMP_DIR}/output_regression_fail_p99.txt" 2>&1; then
    echo "  FAIL: Gate should have failed when p99 exceeds revert threshold"
    cat "${TEMP_DIR}/output_regression_fail_p99.txt"
    exit 1
else
    EXIT_CODE=$?
    if [[ ${EXIT_CODE} -ne 3 ]]; then
        echo "  FAIL: Gate failed with unexpected exit code ${EXIT_CODE} (expected 3)"
        cat "${TEMP_DIR}/output_regression_fail_p99.txt"
        exit 1
    fi

    echo "  PASS: Gate failed with exit 3 as expected (p99=10000ms > 9550ms)"
fi

echo ""
echo "Test 13: FAIL case (regression guard fails with rps=300 below revert)"
mkdir -p "${TEMP_DIR}/regression_fail_rps_case"
cat > "${TEMP_DIR}/regression_fail_rps_case/meta.json" <<'EOF'
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
      "p99": 5000.0
    },
    "error_rate": 0.0,
    "rps": 300,
    "merge_path_detail": {
      "bulk_with_arena": 950,
      "bulk_without_arena": 50,
      "bulk_with_arena_ratio": 0.95
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/regression_fail_rps_case" "tasks_bulk" > "${TEMP_DIR}/output_regression_fail_rps.txt" 2>&1; then
    echo "  FAIL: Gate should have failed when rps is below revert threshold"
    cat "${TEMP_DIR}/output_regression_fail_rps.txt"
    exit 1
else
    EXIT_CODE=$?
    if [[ ${EXIT_CODE} -ne 3 ]]; then
        echo "  FAIL: Gate failed with unexpected exit code ${EXIT_CODE} (expected 3)"
        cat "${TEMP_DIR}/output_regression_fail_rps.txt"
        exit 1
    fi

    echo "  PASS: Gate failed with exit 3 as expected (rps=300 < 341.36)"
fi

echo ""
echo "Test 14: FAIL case (regression guard fails with both p99 and rps violations)"
mkdir -p "${TEMP_DIR}/regression_fail_both_case"
cat > "${TEMP_DIR}/regression_fail_both_case/meta.json" <<'EOF'
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
      "p99": 15000.0
    },
    "error_rate": 0.0,
    "rps": 200,
    "merge_path_detail": {
      "bulk_with_arena": 950,
      "bulk_without_arena": 50,
      "bulk_with_arena_ratio": 0.95
    }
  }
}
EOF

if "${CHECK_THRESHOLDS}" "${TEMP_DIR}/regression_fail_both_case" "tasks_bulk" > "${TEMP_DIR}/output_regression_fail_both.txt" 2>&1; then
    echo "  FAIL: Gate should have failed when both p99 and rps violate revert thresholds"
    cat "${TEMP_DIR}/output_regression_fail_both.txt"
    exit 1
else
    EXIT_CODE=$?
    if [[ ${EXIT_CODE} -ne 3 ]]; then
        echo "  FAIL: Gate failed with unexpected exit code ${EXIT_CODE} (expected 3)"
        cat "${TEMP_DIR}/output_regression_fail_both.txt"
        exit 1
    fi

    echo "  PASS: Gate failed with exit 3 as expected (p99=15000ms > 9550ms, rps=200 < 341.36)"
fi

echo ""
echo "All tests passed!"
