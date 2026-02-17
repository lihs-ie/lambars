#!/usr/bin/env bash
# Test suite for validate_profiling_artifacts.sh
# Tests: stacks.folded and flamegraph.svg artifact validation

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_SCRIPT="${SCRIPT_DIR}/validate_profiling_artifacts.sh"

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

assert_exit_code() {
    local expected="$1"
    local actual="$2"
    local test_name="$3"
    ((TESTS_RUN++))
    if [[ "${expected}" == "${actual}" ]]; then
        log_pass "${test_name}"
        ((TESTS_PASSED++))
    else
        log_fail "${test_name} (expected exit=${expected}, got exit=${actual})"
        ((TESTS_FAILED++))
        return 1
    fi
}

assert_contains() {
    local pattern="$1"
    local text="$2"
    local test_name="$3"
    ((TESTS_RUN++))
    if echo "${text}" | grep -qF "${pattern}"; then
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

# -------------------------------------------------------------------
# Fixtures
# -------------------------------------------------------------------

create_valid_stacks_folded() {
    local directory="$1"
    cat > "${directory}/stacks.folded" << 'EOF'
effect_bench;<futures_util::future::future::catch_unwind::CatchUnwind<Fut> as core::future::future::Future>::poll 387161474
effect_bench;<futures_util::stream::futures_unordered::FuturesUnordered<Fut> as futures_core::stream::Stream>::poll_next 532597779
lambars::effect::async_io::AsyncIo::run_io;tokio::runtime::task::harness::poll_future 1234567890
EOF
}

create_valid_flamegraph_svg() {
    local directory="$1"
    cat > "${directory}/flamegraph.svg" << 'EOF'
<?xml version="1.0" standalone="no"?><!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd"><svg version="1.1" width="1200" height="422" onload="init(evt)" viewBox="0 0 1200 422" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><!-- Flamegraph content --></svg>
EOF
}

create_stacks_folded_with_perf_not_found() {
    local directory="$1"
    cat > "${directory}/stacks.folded" << 'EOF'
[WARN  inferno::collapse::perf::logging] Weird event line: WARNING: perf not found for kernel 6.14.0-1017

thread 'main' (4675) panicked at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/inferno-0.12.4/src/collapse/perf.rs:191:9:
assertion failed: self.event_filter.is_some()
EOF
}

create_stacks_folded_with_assertion_failed() {
    local directory="$1"
    cat > "${directory}/stacks.folded" << 'EOF'
some_frame;inner_frame 100
thread 'main' panicked at some_file.rs:42:
assertion failed: self.is_valid()
EOF
}

create_stacks_folded_empty_counts() {
    local directory="$1"
    # Has content but no valid "<stack> <count>" lines
    cat > "${directory}/stacks.folded" << 'EOF'
# This is just a comment
# No valid stack count lines here
EOF
}

create_flamegraph_svg_with_no_valid_input() {
    local directory="$1"
    cat > "${directory}/flamegraph.svg" << 'EOF'
<?xml version="1.0" standalone="no"?>ERROR: No valid input provided to flamegraph<svg version="1.1" width="1200" height="60"></svg>
EOF
}

create_flamegraph_svg_with_no_stack_counts() {
    local directory="$1"
    cat > "${directory}/flamegraph.svg" << 'EOF'
[ERROR inferno::flamegraph] No stack counts found
Error: Custom { kind: InvalidData, error: "No stack counts found" }
<?xml version="1.0" standalone="no"?>ERROR: No stack counts found<svg></svg>
EOF
}

create_flamegraph_svg_with_error_prefix() {
    local directory="$1"
    cat > "${directory}/flamegraph.svg" << 'EOF'
<?xml version="1.0" standalone="no"?>
<svg version="1.1" width="1200" height="422">
ERROR: some rendering error occurred in flamegraph
</svg>
EOF
}

# -------------------------------------------------------------------
# Test: Script exists and is executable
# -------------------------------------------------------------------
test_script_exists() {
    cat << 'EOF'

==============================================
  Testing: script existence
==============================================
EOF
    ((TESTS_RUN++))
    if [[ -f "${TARGET_SCRIPT}" && -x "${TARGET_SCRIPT}" ]]; then
        log_pass "validate_profiling_artifacts.sh exists and is executable"
        ((TESTS_PASSED++))
    else
        log_fail "validate_profiling_artifacts.sh missing or not executable"
        ((TESTS_FAILED++))
    fi
}

# -------------------------------------------------------------------
# Test: Valid artifacts pass
# -------------------------------------------------------------------
test_valid_artifacts_pass() {
    cat << 'EOF'

==============================================
  Testing: valid artifacts pass
==============================================
EOF
    local tmpdir
    tmpdir=$(mktemp -d)

    create_valid_stacks_folded "${tmpdir}"
    create_valid_flamegraph_svg "${tmpdir}"

    local output exit_code
    output=$("${TARGET_SCRIPT}" "${tmpdir}" 2>&1) && exit_code=0 || exit_code=$?

    assert_exit_code 0 "${exit_code}" "valid artifacts: exit code 0"
    assert_contains "PASS" "${output}" "valid artifacts: output contains PASS"

    rm -rf "${tmpdir}"
}

# -------------------------------------------------------------------
# Test: stacks.folded with "perf not found" fails
# -------------------------------------------------------------------
test_stacks_folded_perf_not_found_fails() {
    cat << 'EOF'

==============================================
  Testing: stacks.folded with 'perf not found'
==============================================
EOF
    local tmpdir
    tmpdir=$(mktemp -d)

    create_stacks_folded_with_perf_not_found "${tmpdir}"
    create_valid_flamegraph_svg "${tmpdir}"

    local output
    local exit_code
    output=$("${TARGET_SCRIPT}" "${tmpdir}" 2>&1) && exit_code=0 || exit_code=$?

    assert_exit_code 1 "${exit_code}" "perf not found: exit code 1"
    assert_contains "FAIL" "${output}" "perf not found: output contains FAIL"

    rm -rf "${tmpdir}"
}

# -------------------------------------------------------------------
# Test: stacks.folded with "assertion failed" fails
# -------------------------------------------------------------------
test_stacks_folded_assertion_failed_fails() {
    cat << 'EOF'

==============================================
  Testing: stacks.folded with 'assertion failed'
==============================================
EOF
    local tmpdir
    tmpdir=$(mktemp -d)

    create_stacks_folded_with_assertion_failed "${tmpdir}"
    create_valid_flamegraph_svg "${tmpdir}"

    local output
    local exit_code
    output=$("${TARGET_SCRIPT}" "${tmpdir}" 2>&1) && exit_code=0 || exit_code=$?

    assert_exit_code 1 "${exit_code}" "assertion failed: exit code 1"
    assert_contains "FAIL" "${output}" "assertion failed: output contains FAIL"

    rm -rf "${tmpdir}"
}

# -------------------------------------------------------------------
# Test: stacks.folded without valid stack count lines fails
# -------------------------------------------------------------------
test_stacks_folded_no_valid_counts_fails() {
    cat << 'EOF'

==============================================
  Testing: stacks.folded without valid count lines
==============================================
EOF
    local tmpdir
    tmpdir=$(mktemp -d)

    create_stacks_folded_empty_counts "${tmpdir}"
    create_valid_flamegraph_svg "${tmpdir}"

    local output
    local exit_code
    output=$("${TARGET_SCRIPT}" "${tmpdir}" 2>&1) && exit_code=0 || exit_code=$?

    assert_exit_code 1 "${exit_code}" "no valid counts: exit code 1"
    assert_contains "FAIL" "${output}" "no valid counts: output contains FAIL"

    rm -rf "${tmpdir}"
}

# -------------------------------------------------------------------
# Test: flamegraph.svg with "No valid input provided" fails
# -------------------------------------------------------------------
test_flamegraph_svg_no_valid_input_fails() {
    cat << 'EOF'

==============================================
  Testing: flamegraph.svg with 'No valid input provided'
==============================================
EOF
    local tmpdir
    tmpdir=$(mktemp -d)

    create_valid_stacks_folded "${tmpdir}"
    create_flamegraph_svg_with_no_valid_input "${tmpdir}"

    local output
    local exit_code
    output=$("${TARGET_SCRIPT}" "${tmpdir}" 2>&1) && exit_code=0 || exit_code=$?

    assert_exit_code 1 "${exit_code}" "no valid input: exit code 1"
    assert_contains "FAIL" "${output}" "no valid input: output contains FAIL"

    rm -rf "${tmpdir}"
}

# -------------------------------------------------------------------
# Test: flamegraph.svg with "No stack counts found" fails
# -------------------------------------------------------------------
test_flamegraph_svg_no_stack_counts_fails() {
    cat << 'EOF'

==============================================
  Testing: flamegraph.svg with 'No stack counts found'
==============================================
EOF
    local tmpdir
    tmpdir=$(mktemp -d)

    create_valid_stacks_folded "${tmpdir}"
    create_flamegraph_svg_with_no_stack_counts "${tmpdir}"

    local output
    local exit_code
    output=$("${TARGET_SCRIPT}" "${tmpdir}" 2>&1) && exit_code=0 || exit_code=$?

    assert_exit_code 1 "${exit_code}" "no stack counts found: exit code 1"
    assert_contains "FAIL" "${output}" "no stack counts found: output contains FAIL"

    rm -rf "${tmpdir}"
}

# -------------------------------------------------------------------
# Test: flamegraph.svg with "ERROR:" prefix fails
# -------------------------------------------------------------------
test_flamegraph_svg_error_prefix_fails() {
    cat << 'EOF'

==============================================
  Testing: flamegraph.svg with 'ERROR:' prefix
==============================================
EOF
    local tmpdir
    tmpdir=$(mktemp -d)

    create_valid_stacks_folded "${tmpdir}"
    create_flamegraph_svg_with_error_prefix "${tmpdir}"

    local output
    local exit_code
    output=$("${TARGET_SCRIPT}" "${tmpdir}" 2>&1) && exit_code=0 || exit_code=$?

    assert_exit_code 1 "${exit_code}" "ERROR: prefix: exit code 1"
    assert_contains "FAIL" "${output}" "ERROR: prefix: output contains FAIL"

    rm -rf "${tmpdir}"
}

# -------------------------------------------------------------------
# Test: --all mode with multiple directories
# -------------------------------------------------------------------
test_all_mode_multiple_directories() {
    cat << 'EOF'

==============================================
  Testing: --all mode with multiple directories
==============================================
EOF
    local rootdir
    rootdir=$(mktemp -d)

    local valid_dir="${rootdir}/valid_scenario"
    local invalid_dir="${rootdir}/invalid_scenario"

    mkdir -p "${valid_dir}" "${invalid_dir}"

    create_valid_stacks_folded "${valid_dir}"
    create_valid_flamegraph_svg "${valid_dir}"

    create_stacks_folded_with_perf_not_found "${invalid_dir}"
    create_valid_flamegraph_svg "${invalid_dir}"

    local output exit_code
    output=$("${TARGET_SCRIPT}" --all "${rootdir}" 2>&1) && exit_code=0 || exit_code=$?

    assert_exit_code 1 "${exit_code}" "--all mode with invalid: exit code 1"
    assert_contains "FAIL" "${output}" "--all mode: output contains FAIL"
    assert_contains "PASS" "${output}" "--all mode: output contains PASS"

    rm -rf "${rootdir}"
}

# -------------------------------------------------------------------
# Test: --all mode all pass
# -------------------------------------------------------------------
test_all_mode_all_pass() {
    cat << 'EOF'

==============================================
  Testing: --all mode all pass
==============================================
EOF
    local rootdir
    rootdir=$(mktemp -d)

    local dir1="${rootdir}/scenario_a"
    local dir2="${rootdir}/scenario_b"

    mkdir -p "${dir1}" "${dir2}"

    create_valid_stacks_folded "${dir1}"
    create_valid_flamegraph_svg "${dir1}"

    create_valid_stacks_folded "${dir2}"
    create_valid_flamegraph_svg "${dir2}"

    local output exit_code
    output=$("${TARGET_SCRIPT}" --all "${rootdir}" 2>&1) && exit_code=0 || exit_code=$?

    assert_exit_code 0 "${exit_code}" "--all mode all pass: exit code 0"
    assert_contains "PASS" "${output}" "--all mode all pass: output contains PASS"

    rm -rf "${rootdir}"
}

# -------------------------------------------------------------------
# Test: --report option generates report file
# -------------------------------------------------------------------
test_report_option() {
    cat << 'EOF'

==============================================
  Testing: --report option
==============================================
EOF
    local rootdir
    rootdir=$(mktemp -d)

    local dir="${rootdir}/scenario"
    local report_file="${rootdir}/report.txt"

    mkdir -p "${dir}"
    create_valid_stacks_folded "${dir}"
    create_valid_flamegraph_svg "${dir}"

    "${TARGET_SCRIPT}" --all "${rootdir}" --report "${report_file}" > /dev/null 2>&1 || true

    ((TESTS_RUN++))
    if [[ -f "${report_file}" ]]; then
        log_pass "--report generates report file"
        ((TESTS_PASSED++))
    else
        log_fail "--report: report file not created"
        ((TESTS_FAILED++))
    fi

    rm -rf "${rootdir}"
}

# -------------------------------------------------------------------
# Main
# -------------------------------------------------------------------
main() {
    cat << 'EOF'

==============================================
  validate_profiling_artifacts.sh Test Suite
==============================================
EOF

    test_script_exists
    test_valid_artifacts_pass
    test_stacks_folded_perf_not_found_fails
    test_stacks_folded_assertion_failed_fails
    test_stacks_folded_no_valid_counts_fails
    test_flamegraph_svg_no_valid_input_fails
    test_flamegraph_svg_no_stack_counts_fails
    test_flamegraph_svg_error_prefix_fails
    test_all_mode_multiple_directories
    test_all_mode_all_pass
    test_report_option

    cat << EOF

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
