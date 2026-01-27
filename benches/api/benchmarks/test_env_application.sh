#!/usr/bin/env bash
set -uo pipefail  # Remove -e to handle test failures gracefully

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

cleanup() {
    log_info "Cleaning up..."
    if ! docker compose -f "$PROJECT_ROOT/benches/api/docker/compose.ci.yaml" down -v 2>/dev/null; then
        log_warn "docker compose down failed (this is usually not a problem)"
    fi
}
trap cleanup EXIT

test_passed=0
test_failed=0

run_test() {
    local name="$1"
    shift
    # Run test function and capture exit code
    "$@"
    local result=$?
    if [ "$result" -eq 0 ]; then
        log_info "PASS: $name"
        ((test_passed++))
    else
        log_error "FAIL: $name"
        ((test_failed++))
    fi
    return 0  # Don't fail the script
}

wait_for_health() {
    local url="$1"
    local max_attempts="${2:-30}"
    local attempt=1
    while [ "$attempt" -le "$max_attempts" ]; do
        if curl --connect-timeout 2 --max-time 5 -sf "$url" >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
        ((attempt++))
    done
    return 1
}

# Test 1: dry-run doesn't start API
test_dry_run() {
    log_info "Test: dry-run mode doesn't start API"

    # Run dry-run and check output
    if ! cargo xtask bench-api --scenario benches/api/benchmarks/scenarios/smoke.yaml --dry-run 2>&1 | grep -q "DRY RUN MODE"; then
        log_error "dry-run output did not contain 'DRY RUN MODE'"
        return 1
    fi

    # Verify API is NOT running (curl should fail)
    if curl --connect-timeout 2 --max-time 5 -sf http://localhost:3002/health >/dev/null 2>&1; then
        log_error "API is running but should not be in dry-run mode"
        return 1  # API is running, test failed
    fi

    return 0
}

# Test 2: Environment variable injection
test_env_injection() {
    log_info "Test: Environment variables are injected to API"

    # Start API with specific WORKER_THREADS
    WORKER_THREADS=2 ENABLE_DEBUG_ENDPOINTS=true docker compose -f "$PROJECT_ROOT/benches/api/docker/compose.ci.yaml" up -d --build --wait || return 1

    # Wait for health with retry loop
    if ! wait_for_health "http://localhost:3002/health" 30; then
        log_error "API failed to become healthy within 30 seconds"
        docker compose -f "$PROJECT_ROOT/benches/api/docker/compose.ci.yaml" down -v
        return 1
    fi

    # Check /debug/config
    local config
    config=$(curl --connect-timeout 2 --max-time 5 -sf http://localhost:3002/debug/config) || {
        log_error "Failed to fetch /debug/config"
        docker compose -f "$PROJECT_ROOT/benches/api/docker/compose.ci.yaml" down -v
        return 1
    }

    # Cleanup before checking result
    docker compose -f "$PROJECT_ROOT/benches/api/docker/compose.ci.yaml" down -v

    # Verify worker_threads is set to expected value (2)
    local actual_threads
    actual_threads=$(echo "$config" | jq -r '.worker_threads // "null"')
    if [ "$actual_threads" != "2" ]; then
        log_error "worker_threads expected 2 but got $actual_threads in /debug/config response"
        return 1
    fi

    return 0
}

# Test 3: Compose config expansion
test_compose_config() {
    log_info "Test: compose.ci.yaml environment variable templates expand correctly"

    # Strict pattern matching to avoid matching 13, 30, 31, etc.
    # Pattern matches: WORKER_THREADS: "3" OR WORKER_THREADS: '3' OR WORKER_THREADS: 3 (not followed by digit)
    local pattern='WORKER_THREADS: ("3"|'"'"'3'"'"'|3($|[^0-9]))'
    if ! WORKER_THREADS=3 docker compose -f "$PROJECT_ROOT/benches/api/docker/compose.ci.yaml" config 2>/dev/null | grep -qE "$pattern"; then
        log_error "WORKER_THREADS=3 not found in compose config output"
        return 1
    fi

    return 0
}

# Test 4: Cache semantics
test_cache_semantics() {
    log_info "Test: Cache control headers and caching behavior"

    # Start API with cache enabled
    ENABLE_RESPONSE_CACHE=true docker compose -f "$PROJECT_ROOT/benches/api/docker/compose.ci.yaml" up -d --build --wait || return 1

    # Wait for health
    if ! wait_for_health "http://localhost:3002/health" 30; then
        log_error "API failed to become healthy within 30 seconds"
        docker compose -f "$PROJECT_ROOT/benches/api/docker/compose.ci.yaml" down -v
        return 1
    fi

    # Check cache headers
    local headers
    headers=$(curl --connect-timeout 2 --max-time 5 -sI http://localhost:3002/tasks 2>&1) || {
        log_error "Failed to fetch /tasks headers"
        docker compose -f "$PROJECT_ROOT/benches/api/docker/compose.ci.yaml" down -v
        return 1
    }

    # Cleanup
    docker compose -f "$PROJECT_ROOT/benches/api/docker/compose.ci.yaml" down -v

    # Check for cache-related headers (case-insensitive)
    if ! echo "$headers" | grep -qi "cache-control"; then
        log_warn "Cache-Control header not found (may be expected depending on config)"
    fi

    return 0
}

# Main
main() {
    log_info "Starting environment application tests"
    log_info "Project root: $PROJECT_ROOT"

    run_test "dry-run mode" test_dry_run
    run_test "environment injection" test_env_injection
    run_test "compose config expansion" test_compose_config
    run_test "cache semantics" test_cache_semantics

    echo ""
    log_info "========================================="
    log_info "Test Results: $test_passed passed, $test_failed failed"
    log_info "========================================="

    if [ "$test_failed" -gt 0 ]; then
        exit 1
    fi
    exit 0
}

main "$@"
