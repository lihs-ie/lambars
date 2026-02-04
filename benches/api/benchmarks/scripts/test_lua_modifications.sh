#!/usr/bin/env bash
# =============================================================================
# test_lua_modifications.sh - Lua script modification test
# =============================================================================
# Verify that modified Lua scripts do not have syntax errors and can be loaded.
#
# Usage:
#   ./test_lua_modifications.sh
# =============================================================================

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

# =============================================================================
# Test Lua syntax by checking wrk2 can load it
# =============================================================================
test_lua_syntax() {
    local script_path="$1"
    local script_name
    script_name=$(basename "${script_path}")

    log_info "Testing: ${script_name}"

    # wrk2 のヘルプメッセージにスクリプトを読み込ませて構文エラーを検出
    # -s オプションでスクリプトを指定すると、wrk2 は構文チェックを行う
    # ただし、実際のサーバーがないためエラーが出るが、構文エラーは検出可能
    if ! command -v wrk2 >/dev/null 2>&1; then
        log_error "wrk2 not found. Cannot test Lua syntax."
        return 1
    fi

    # wrk2 で構文エラーを検出（実際のベンチマークは実行しない）
    # -d0s で即座に終了させる
    local output
    output=$(cd "${SCRIPT_DIR}" && wrk2 -t1 -c1 -d0s -s "${script_name}" http://localhost:1 2>&1 || true)

    # Lua スクリプトエラーを検出
    if echo "${output}" | grep -iqE "attempt to|unexpected symbol|syntax error|\.lua:[0-9]+:"; then
        log_error "Syntax error detected in ${script_name}"
        echo "${output}"
        return 1
    fi

    log_info "Syntax OK: ${script_name}"
    return 0
}

# =============================================================================
# Main function
# =============================================================================
main() {
    log_info "=== Lua Script Modification Test ==="

    local modified_scripts=(
        "tasks_update.lua"
    )

    local passed=0
    local failed=0

    for script in "${modified_scripts[@]}"; do
        if test_lua_syntax "${script}"; then
            passed=$((passed + 1))
        else
            failed=$((failed + 1))
        fi
    done

    # Summary
    echo ""
    log_info "=== Test Summary ==="
    log_info "Passed: ${passed}"
    if [[ ${failed} -gt 0 ]]; then
        log_error "Failed: ${failed}"
        exit 1
    else
        log_info "Failed: 0"
        log_info "All modified scripts have valid syntax."
    fi
}

main "$@"
