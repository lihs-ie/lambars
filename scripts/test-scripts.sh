#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_RESULTS_DIR="$SCRIPT_DIR/../benches/results/script-tests"
mkdir -p "$TEST_RESULTS_DIR"

echo "=== Shell Script Test Suite ==="
echo "Script directory: $SCRIPT_DIR"
echo ""

TESTS_RUN=0 TESTS_PASSED=0 TESTS_FAILED=0

run_test() {
    local test_name="$1"
    shift
    TESTS_RUN=$((TESTS_RUN + 1))
    echo "Test $TESTS_RUN: $test_name"
    if "$@"; then
        echo "✓ PASSED"; TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo "✗ FAILED"; TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    echo ""
}

run_test "benchmark-env.sh --help" \
    bash "$SCRIPT_DIR/benchmark-env.sh" --help

run_test "inline-experiment.sh --help" \
    "$SCRIPT_DIR/inline-experiment.sh" --help

run_test "lto-cgu-experiment.sh --help" \
    "$SCRIPT_DIR/lto-cgu-experiment.sh" --help

run_test "cold-experiment.sh --help" \
    "$SCRIPT_DIR/cold-experiment.sh" --help

run_test "benchmark-stats.sh --help" \
    "$SCRIPT_DIR/benchmark-stats.sh" --help

TEST_CSV="$TEST_RESULTS_DIR/test-data.csv"
cat > "$TEST_CSV" <<EOF
run,benchmark,instructions
1,push_back_1000,312708
2,push_back_1000,312800
3,push_back_1000,312650
4,push_back_1000,312750
5,push_back_1000,312700
EOF

run_test "benchmark-stats.sh with test data" \
    "$SCRIPT_DIR/benchmark-stats.sh" --input "$TEST_CSV" --output "$TEST_RESULTS_DIR/stats-output.txt"

test_stats_output_validation() {
    grep -q 'Mean:' "$TEST_RESULTS_DIR/stats-output.txt" && \
    grep -q 'Std Dev:' "$TEST_RESULTS_DIR/stats-output.txt" && \
    grep -q 'CV:' "$TEST_RESULTS_DIR/stats-output.txt"
}
run_test "benchmark-stats.sh output validation" \
    test_stats_output_validation

test_benchmark_env_valid_lto() {
    (source "$SCRIPT_DIR/benchmark-env.sh" --lto thin && [[ $BENCH_LTO == 'thin' ]])
}
run_test "benchmark-env.sh valid LTO" \
    bash -c "source '$SCRIPT_DIR/benchmark-env.sh' --lto thin && [[ \$BENCH_LTO == 'thin' ]]"

test_benchmark_env_invalid_lto() {
    ! bash "$SCRIPT_DIR/benchmark-env.sh" --lto invalid 2>/dev/null
}
run_test "benchmark-env.sh invalid LTO" \
    test_benchmark_env_invalid_lto

test_benchmark_env_valid_cgu() {
    (source "$SCRIPT_DIR/benchmark-env.sh" --cgu 16 && [[ $BENCH_CGU == '16' ]])
}
run_test "benchmark-env.sh valid CGU" \
    bash -c "source '$SCRIPT_DIR/benchmark-env.sh' --cgu 16 && [[ \$BENCH_CGU == '16' ]]"

test_benchmark_env_invalid_cgu() {
    ! bash "$SCRIPT_DIR/benchmark-env.sh" --cgu 0 2>/dev/null
}
run_test "benchmark-env.sh invalid CGU (0)" \
    test_benchmark_env_invalid_cgu

test_benchmark_env_missing_arg_lto() {
    ! bash "$SCRIPT_DIR/benchmark-env.sh" --lto 2>/dev/null
}
run_test "benchmark-env.sh missing argument for --lto" \
    test_benchmark_env_missing_arg_lto

test_benchmark_env_missing_arg_cgu() {
    ! bash "$SCRIPT_DIR/benchmark-env.sh" --cgu 2>/dev/null
}
run_test "benchmark-env.sh missing argument for --cgu" \
    test_benchmark_env_missing_arg_cgu

test_benchmark_stats_missing_input() {
    ! "$SCRIPT_DIR/benchmark-stats.sh" --output "$TEST_RESULTS_DIR/dummy.txt" 2>/dev/null
}
run_test "benchmark-stats.sh missing --input" \
    test_benchmark_stats_missing_input

test_benchmark_stats_invalid_file() {
    ! "$SCRIPT_DIR/benchmark-stats.sh" --input "/nonexistent/file.csv" 2>/dev/null
}
run_test "benchmark-stats.sh invalid input file" \
    test_benchmark_stats_invalid_file

test_inline_experiment_missing_arg() {
    ! "$SCRIPT_DIR/inline-experiment.sh" --baseline 2>/dev/null
}
run_test "inline-experiment.sh missing argument for --baseline" \
    test_inline_experiment_missing_arg

test_lto_cgu_experiment_missing_arg() {
    ! "$SCRIPT_DIR/lto-cgu-experiment.sh" --bench 2>/dev/null
}
run_test "lto-cgu-experiment.sh missing argument for --bench" \
    test_lto_cgu_experiment_missing_arg

test_cold_experiment_missing_arg() {
    ! "$SCRIPT_DIR/cold-experiment.sh" --runs 2>/dev/null
}
run_test "cold-experiment.sh missing argument for --runs" \
    test_cold_experiment_missing_arg

test_cold_experiment_invalid_runs_zero() {
    ! "$SCRIPT_DIR/cold-experiment.sh" --runs 0 2>/dev/null
}
run_test "cold-experiment.sh invalid runs (0)" \
    test_cold_experiment_invalid_runs_zero

test_cold_experiment_invalid_runs_below_10() {
    ! "$SCRIPT_DIR/cold-experiment.sh" --runs 5 2>/dev/null
}
run_test "cold-experiment.sh invalid runs (< 10)" \
    test_cold_experiment_invalid_runs_below_10

echo "=== Summary ==="
echo "Run: $TESTS_RUN / Passed: $TESTS_PASSED / Failed: $TESTS_FAILED"
echo ""

if [[ $TESTS_FAILED -eq 0 ]]; then
    echo "✓ All tests passed"
    exit 0
else
    echo "✗ Some tests failed"
    exit 1
fi
