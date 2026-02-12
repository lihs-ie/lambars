#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../../../.." && pwd)"
CHECK_THRESHOLDS="${PROJECT_ROOT}/benches/api/benchmarks/check_thresholds.sh"
TEMP_DIR="${PROJECT_ROOT}/benches/api/benchmarks/scripts/temp_test_merge_path_gate"

trap 'rm -rf "${TEMP_DIR}"' EXIT
mkdir -p "${TEMP_DIR}"

create_meta_json() {
    local dir="$1"
    local p99="${2:-20.0}"
    local rps="${3:-400}"
    local merge_with="${4:-}"
    local merge_without="${5:-}"
    local merge_ratio="${6:-}"
    local merge_error="${7:-}"

    mkdir -p "${dir}"
    cat > "${dir}/meta.json" <<EOF
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
      "p99": ${p99}
    },
    "error_rate": 0.0,
    "rps": ${rps}
EOF

    if [[ -n "${merge_with}" ]] || [[ -n "${merge_error}" ]]; then
        echo '    ,"merge_path_detail": {' >> "${dir}/meta.json"
        if [[ -n "${merge_error}" ]]; then
            echo "      \"merge_path_error\": \"${merge_error}\"" >> "${dir}/meta.json"
        else
            cat >> "${dir}/meta.json" <<EOF
      "bulk_with_arena": ${merge_with},
      "bulk_without_arena": ${merge_without},
      "bulk_with_arena_ratio": ${merge_ratio}
EOF
        fi
        echo '    }' >> "${dir}/meta.json"
    fi

    cat >> "${dir}/meta.json" <<'EOF'
  }
}
EOF
}

run_test_expect_pass() {
    local test_name="$1"
    local dir="$2"
    local scenario="${3:-tasks_bulk}"
    shift 3
    local patterns=("$@")

    if ! "${CHECK_THRESHOLDS}" "${dir}" "${scenario}" > "${TEMP_DIR}/output.txt" 2>&1; then
        echo "  FAIL: ${test_name} - gate should have passed"
        cat "${TEMP_DIR}/output.txt"
        exit 1
    fi

    for pattern in "${patterns[@]}"; do
        if ! grep -q "$pattern" "${TEMP_DIR}/output.txt"; then
            echo "  ERROR: ${test_name} - expected pattern not found: $pattern"
            cat "${TEMP_DIR}/output.txt"
            exit 1
        fi
    done

    echo "  PASS: ${test_name}"
}

run_test_expect_fail() {
    local test_name="$1"
    local dir="$2"
    local expected_exit="$3"
    local scenario="${4:-tasks_bulk}"
    shift 4
    local patterns=("$@")

    set +e
    "${CHECK_THRESHOLDS}" "${dir}" "${scenario}" > "${TEMP_DIR}/output.txt" 2>&1
    local actual_exit=$?
    set -e

    if [[ ${actual_exit} -eq 0 ]]; then
        echo "  FAIL: ${test_name} - gate should have failed"
        cat "${TEMP_DIR}/output.txt"
        exit 1
    fi

    if [[ ${actual_exit} -ne ${expected_exit} ]]; then
        echo "  FAIL: ${test_name} - unexpected exit code ${actual_exit} (expected ${expected_exit})"
        cat "${TEMP_DIR}/output.txt"
        exit 1
    fi

    for pattern in "${patterns[@]}"; do
        if ! grep -q "$pattern" "${TEMP_DIR}/output.txt"; then
            echo "  ERROR: ${test_name} - expected pattern not found: $pattern"
            cat "${TEMP_DIR}/output.txt"
            exit 1
        fi
    done

    echo "  PASS: ${test_name}"
}

run_test_not_contain() {
    local test_name="$1"
    local dir="$2"
    local scenario="$3"
    local not_pattern="$4"

    if ! "${CHECK_THRESHOLDS}" "${dir}" "${scenario}" > "${TEMP_DIR}/output.txt" 2>&1; then
        echo "  FAIL: ${test_name} - gate should have passed"
        cat "${TEMP_DIR}/output.txt"
        exit 1
    fi

    if grep -q "$not_pattern" "${TEMP_DIR}/output.txt"; then
        echo "  ERROR: ${test_name} - unexpected pattern found: $not_pattern"
        cat "${TEMP_DIR}/output.txt"
        exit 1
    fi

    echo "  PASS: ${test_name}"
}

echo "Test 1: PASS case (bulk_with_arena_ratio = 0.95, tasks_bulk scenario)"
create_meta_json "${TEMP_DIR}/pass_case" 20.0 400 950 50 0.95
run_test_expect_pass "Gate passed as expected" "${TEMP_DIR}/pass_case" "tasks_bulk" \
    "bulk_with_arena_ratio = 0.950000000" \
    "PASS: Merge path telemetry within acceptable range"

echo ""
echo "Test 2: FAIL case (bulk_with_arena_ratio = 0.50, tasks_bulk scenario)"
create_meta_json "${TEMP_DIR}/fail_case" 20.0 400 500 500 0.50
run_test_expect_fail "Gate failed with exit 3 as expected" "${TEMP_DIR}/fail_case" 3 "tasks_bulk" \
    "FAIL: Merge path regression detected" \
    "bulk_with_arena_ratio = 0.500000000"

echo ""
echo "Test 3: FAIL case (merge_path_detail missing, tasks_bulk scenario)"
create_meta_json "${TEMP_DIR}/fail_missing_case" 20.0 400
run_test_expect_fail "Gate failed with exit 2 as expected (merge_path_detail missing)" \
    "${TEMP_DIR}/fail_missing_case" 2 "tasks_bulk" \
    "FAIL.*merge_path_detail not found or not an object"

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
run_test_not_contain "Gate passed (merge_path check skipped for tasks_update)" \
    "${TEMP_DIR}/skip_case" "tasks_update" "Merge path telemetry"

echo ""
echo "Test 5: Boundary case (bulk_with_arena_ratio = 0.90 exactly, should PASS)"
create_meta_json "${TEMP_DIR}/boundary_case" 20.0 400 900 100 0.90
run_test_expect_pass "Gate passed at boundary (ratio = 0.90)" "${TEMP_DIR}/boundary_case" "tasks_bulk" \
    "bulk_with_arena_ratio = 0.900000000"

echo ""
echo "Test 6: Ratio mismatch case (stored ratio differs from calculated, should use calculated)"
create_meta_json "${TEMP_DIR}/mismatch_case" 20.0 400 920 80 0.50
run_test_expect_pass "Gate passed using recalculated ratio (ignoring stored 0.50)" \
    "${TEMP_DIR}/mismatch_case" "tasks_bulk" \
    "bulk_with_arena_ratio = 0.920000000"

echo ""
echo "Test 7: Reverse mismatch case (stored ratio high but recalculated is low, should FAIL)"
create_meta_json "${TEMP_DIR}/reverse_mismatch_case" 20.0 400 100 900 0.99
run_test_expect_fail "Gate failed with exit 3 as expected (recalculated ratio 0.10 < 0.90)" \
    "${TEMP_DIR}/reverse_mismatch_case" 3 "tasks_bulk" \
    "bulk_with_arena_ratio = 0.100000000"

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
run_test_expect_fail "Gate failed with exit 2 as expected (invalid numeric fields)" \
    "${TEMP_DIR}/invalid_numeric_case" 2 "tasks_bulk" \
    "merge_path_detail.bulk_\\* must be non-negative integers"

echo ""
echo "Test 9: FAIL case (bulk_with_arena and bulk_without_arena are empty strings)"
create_meta_json "${TEMP_DIR}/empty_fields_case" 20.0 400 '""' '""' 0.00
run_test_expect_fail "Gate failed with exit 2 as expected (empty fields)" \
    "${TEMP_DIR}/empty_fields_case" 2 "tasks_bulk" \
    "FAIL.*merge_path_detail fields incomplete"

echo ""
echo "Test 10: FAIL case (merge_path_error field only, no bulk fields)"
create_meta_json "${TEMP_DIR}/merge_path_error_case" 20.0 400 "" "" "" "stacks.folded not found"
run_test_expect_fail "Gate failed with exit 2 as expected (merge_path_error only)" \
    "${TEMP_DIR}/merge_path_error_case" 2 "tasks_bulk" \
    "FAIL.*merge_path_detail fields incomplete"

echo ""
echo "Test 11: PASS case (regression guard passes with p99=50ms, rps=400)"
create_meta_json "${TEMP_DIR}/regression_pass_case" 50.0 400 950 50 0.95
run_test_expect_pass "Gate passed (p99=50ms <= 9550ms, rps=400 >= 341.36)" \
    "${TEMP_DIR}/regression_pass_case" "tasks_bulk"

echo ""
echo "Test 12: FAIL case (regression guard fails with p99=10000ms exceeding revert)"
create_meta_json "${TEMP_DIR}/regression_fail_p99_case" 10000.0 400 950 50 0.95
run_test_expect_fail "Gate failed with exit 3 as expected (p99=10000ms > 9550ms)" \
    "${TEMP_DIR}/regression_fail_p99_case" 3 "tasks_bulk"

echo ""
echo "Test 13: FAIL case (regression guard fails with rps=300 below revert)"
create_meta_json "${TEMP_DIR}/regression_fail_rps_case" 5000.0 300 950 50 0.95
run_test_expect_fail "Gate failed with exit 3 as expected (rps=300 < 341.36)" \
    "${TEMP_DIR}/regression_fail_rps_case" 3 "tasks_bulk"

echo ""
echo "Test 14: FAIL case (regression guard fails with both p99 and rps violations)"
create_meta_json "${TEMP_DIR}/regression_fail_both_case" 15000.0 200 950 50 0.95
run_test_expect_fail "Gate failed with exit 3 as expected (p99=15000ms > 9550ms, rps=200 < 341.36)" \
    "${TEMP_DIR}/regression_fail_both_case" 3 "tasks_bulk"

echo ""
echo "Test 15: Boundary case (p99=9550, rps=341.36 exactly, regression guard PASS but p99 threshold FAIL)"
create_meta_json "${TEMP_DIR}/boundary_regression_case" 9550.0 341.36 950 50 0.95
run_test_expect_fail "Gate failed with exit 3 as expected (p99=9550 exceeds threshold 80ms but passes regression guard)" \
    "${TEMP_DIR}/boundary_regression_case" 3 "tasks_bulk" \
    "PASS: Regression guard within acceptable range" \
    "p99=9550.0ms exceeds threshold of 80ms"

echo ""
echo "Test 16: FAIL case (rps not found in meta.json)"
mkdir -p "${TEMP_DIR}/rps_missing_case"
cat > "${TEMP_DIR}/rps_missing_case/meta.json" <<'EOF'
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
run_test_expect_fail "Gate failed with exit 2 as expected (rps not found)" \
    "${TEMP_DIR}/rps_missing_case" 2 "tasks_bulk" \
    "FAIL.*RPS not found in meta.json"

echo ""
echo "Test 17: FAIL case (rps is not numeric)"
mkdir -p "${TEMP_DIR}/rps_non_numeric_case"
cat > "${TEMP_DIR}/rps_non_numeric_case/meta.json" <<'EOF'
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
    "rps": "abc",
    "merge_path_detail": {
      "bulk_with_arena": 950,
      "bulk_without_arena": 50,
      "bulk_with_arena_ratio": 0.95
    }
  }
}
EOF
run_test_expect_fail "Gate failed with exit 2 as expected (rps is non-numeric)" \
    "${TEMP_DIR}/rps_non_numeric_case" 2 "tasks_bulk" \
    "ERROR.*results.rps must be numeric"

echo ""
echo "Test 18: Near-threshold rounding case (actual < 0.90 must FAIL)"
create_meta_json "${TEMP_DIR}/near_threshold_rounding_case" 20.0 400 2249999 250001 0.99
run_test_expect_fail "Gate fails when unrounded ratio is below threshold (0.8999996 < 0.90)" \
    "${TEMP_DIR}/near_threshold_rounding_case" 3 "tasks_bulk" \
    "FAIL: Merge path regression detected"

echo ""
echo "Test 19: FAIL case (merge_path_detail is non-object type)"
mkdir -p "${TEMP_DIR}/non_object_case"
cat > "${TEMP_DIR}/non_object_case/meta.json" <<'EOF'
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
    "merge_path_detail": "invalid_string"
  }
}
EOF
run_test_expect_fail "Gate failed with exit 2 as expected (non-object merge_path_detail)" \
    "${TEMP_DIR}/non_object_case" 2 "tasks_bulk" \
    "merge_path_detail not found or not an object"

echo ""
echo "All tests passed!"
