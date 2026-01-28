#!/usr/bin/env bash
# =============================================================================
# test_lua_compatibility.sh - Lua script wrk2 compatibility test
# =============================================================================
# Verify that all Lua scripts work correctly with wrk2.
#
# Test contents:
#   1. Run each Lua script with wrk2 for 5 seconds
#   2. Verify completion without errors
#   3. Verify Requests/sec output is present
#
# Prerequisites:
#   - wrk2 must be installed
#   - Target API server must be running (default: localhost:8080)
#
# Usage:
#   ./test_lua_compatibility.sh [--target URL] [--duration SECONDS]
#
# Options:
#   --target URL       Target URL (default: http://localhost:8080)
#   --duration SECONDS Duration per script (default: 5)
#   --skip-server      Skip server availability check
#   --verbose          Verbose output
# =============================================================================

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPTS_DIR="${SCRIPT_DIR}/scripts"
readonly RESULTS_DIR="${SCRIPT_DIR}/results/lua_compatibility"

# Default settings
DEFAULT_TARGET="http://localhost:8080"
DEFAULT_DURATION=5
DEFAULT_RATE=10

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

log_test() {
    echo -e "${BLUE}[TEST]${NC} $*"
}

# =============================================================================
# Check if wrk2 is installed
# =============================================================================
check_wrk2() {
    if ! command -v wrk2 &>/dev/null; then
        log_error "wrk2 is not installed. Please run ./setup_wrk2.sh first."
        exit 1
    fi
    local version
    version=$(wrk2 -v 2>&1 | head -1 || true)
    log_info "Using wrk2: ${version}"
}

# =============================================================================
# Check if server is running
# =============================================================================
check_server() {
    local target="$1"
    local skip_check="$2"

    if [[ "${skip_check}" == "true" ]]; then
        log_warn "Skipping server check"
        return 0
    fi

    log_info "Checking if server is available at ${target}..."

    # Try health check endpoints
    local health_endpoints=("/health" "/api/health" "/" "/ping")

    for endpoint in "${health_endpoints[@]}"; do
        local url="${target}${endpoint}"
        if curl -s -o /dev/null -w "%{http_code}" --connect-timeout 5 "${url}" | grep -q "^[23]"; then
            log_info "Server is available at ${url}"
            return 0
        fi
    done

    log_error "Server is not available at ${target}"
    log_error "Please start the server first:"
    log_error "  cd ../docker && docker compose -f compose.ci.yaml up -d"
    exit 1
}

# =============================================================================
# Test Lua script
# =============================================================================
test_lua_script() {
    local script_path="$1"
    local target="$2"
    local duration="$3"
    local rate="$4"
    local verbose="$5"
    local output_file="$6"

    local script_name
    script_name=$(basename "${script_path}")

    log_test "Testing: ${script_name}"

    # Run wrk2
    # -t1: 1 thread
    # -c1: 1 connection
    # -d: duration
    # -R: request rate (requests/sec)
    # -s: Lua script
    # Note: Run from scripts directory so that Lua require() resolves correctly
    local wrk2_output
    local exit_code=0

    wrk2_output=$(cd "${SCRIPTS_DIR}" && wrk2 -t1 -c1 -d"${duration}s" -R"${rate}" -s "${script_path}" "${target}" 2>&1) || exit_code=$?

    # Save results to file
    echo "=== ${script_name} ===" >> "${output_file}"
    echo "${wrk2_output}" >> "${output_file}"
    echo "" >> "${output_file}"

    # Verbose output
    if [[ "${verbose}" == "true" ]]; then
        echo "${wrk2_output}"
        echo ""
    fi

    # Result validation
    local passed=true

    # 1. Error check (non-zero exit code)
    if [[ ${exit_code} -ne 0 ]]; then
        log_error "  FAILED: wrk2 exited with code ${exit_code}"
        passed=false
    fi

    # 2. Lua script error check
    # Detect typical Lua runtime error patterns:
    # - "attempt to" (attempt to call, attempt to index nil, etc.)
    # - "unexpected symbol"
    # - "syntax error"
    # - ".lua:[0-9]" (filename:line_number format)
    if echo "${wrk2_output}" | grep -iqE "attempt to|unexpected symbol|syntax error|\.lua:[0-9]+:"; then
        log_error "  FAILED: Lua script error detected"
        passed=false
    elif echo "${wrk2_output}" | grep -iq "error"; then
        # "Socket errors" and "Non-2xx or 3xx responses" are treated as warnings
        log_warn "  WARNING: Some errors detected (may be expected)"
    fi

    # 3. Requests/sec output check
    if ! echo "${wrk2_output}" | grep -q "Requests/sec"; then
        log_error "  FAILED: No 'Requests/sec' in output"
        passed=false
    fi

    # 4. Completion message check
    if ! echo "${wrk2_output}" | grep -q "requests in"; then
        log_error "  FAILED: Benchmark did not complete normally"
        passed=false
    fi

    if [[ "${passed}" == "true" ]]; then
        local rps
        rps=$(echo "${wrk2_output}" | grep "Requests/sec" | awk '{print $2}')
        log_info "  PASSED: ${rps} requests/sec"
        return 0
    else
        return 1
    fi
}

# =============================================================================
# Main function
# =============================================================================
main() {
    local target="${DEFAULT_TARGET}"
    local duration="${DEFAULT_DURATION}"
    local rate="${DEFAULT_RATE}"
    local skip_server=false
    local verbose=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --target)
                target="$2"
                shift 2
                ;;
            --duration)
                duration="$2"
                shift 2
                ;;
            --rate)
                rate="$2"
                shift 2
                ;;
            --skip-server)
                skip_server=true
                shift
                ;;
            --verbose|-v)
                verbose=true
                shift
                ;;
            --help|-h)
                echo "Usage: $0 [options]"
                echo ""
                echo "Options:"
                echo "  --target URL       Target URL (default: ${DEFAULT_TARGET})"
                echo "  --duration SECONDS Duration per script (default: ${DEFAULT_DURATION})"
                echo "  --rate RPS         Request rate (default: ${DEFAULT_RATE})"
                echo "  --skip-server      Skip server availability check"
                echo "  --verbose, -v      Verbose output"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    log_info "=== Lua Script wrk2 Compatibility Test ==="
    log_info "Target: ${target}"
    log_info "Duration: ${duration}s per script"
    log_info "Rate: ${rate} req/sec"

    # Prerequisites check
    check_wrk2
    check_server "${target}" "${skip_server}"

    # Create results directory
    mkdir -p "${RESULTS_DIR}"
    local timestamp
    timestamp=$(date +%Y%m%d_%H%M%S)
    local output_file="${RESULTS_DIR}/compatibility_test_${timestamp}.txt"

    # Record header information
    {
        echo "=== Lua Script wrk2 Compatibility Test ==="
        echo "Date: $(date)"
        echo "Target: ${target}"
        echo "Duration: ${duration}s per script"
        echo "Rate: ${rate} req/sec"
        echo "wrk2: $(wrk2 -v 2>&1 | head -1 || true)"
        echo ""
    } > "${output_file}"

    # Get Lua script list (macOS/Linux compatible)
    local scripts=()
    while IFS= read -r script; do
        scripts+=("${script}")
    done < <(find "${SCRIPTS_DIR}" -name "*.lua" -type f | sort)

    local total=${#scripts[@]}
    local passed=0
    local failed=0
    local failed_scripts=()

    log_info "Found ${total} Lua scripts to test"
    echo ""

    # Test each script
    for script in "${scripts[@]}"; do
        if test_lua_script "${script}" "${target}" "${duration}" "${rate}" "${verbose}" "${output_file}"; then
            passed=$((passed + 1))
        else
            failed=$((failed + 1))
            failed_scripts+=("$(basename "${script}")")
        fi
    done

    # Summary
    echo ""
    log_info "=== Test Summary ==="
    log_info "Total:  ${total}"
    log_info "Passed: ${passed}"

    if [[ ${failed} -gt 0 ]]; then
        log_error "Failed: ${failed}"
        log_error "Failed scripts:"
        for script in "${failed_scripts[@]}"; do
            log_error "  - ${script}"
        done
    else
        log_info "Failed: 0"
    fi

    log_info "Results saved to: ${output_file}"

    # Exit with non-zero if any failures
    if [[ ${failed} -gt 0 ]]; then
        exit 1
    fi

    log_info "=== All Lua scripts are wrk2 compatible ==="
}

main "$@"
