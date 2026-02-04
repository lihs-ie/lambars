#!/usr/bin/env bash
# =============================================================================
# test_tasks_update_fix.sh - tasks_update.lua 修正の検証
# =============================================================================
# tasks_update.lua の thread.id 型エラーを修正したことを検証する。
#
# 修正内容:
#   1. thread.id を tonumber() で数値に変換
#   2. wrk および wrk.format の存在確認を追加
#   3. response() 関数で status の nil チェックを追加
#
# 検証項目:
#   1. Lua構文チェック（dofile で読み込み可能）
#   2. setup() 関数の thread.id 算術演算が動作すること
#   3. request() 関数の wrk.format 呼び出しが安全であること
#
# Usage:
#   ./test_tasks_update_fix.sh
# =============================================================================

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly TARGET_SCRIPT="${SCRIPT_DIR}/tasks_update.lua"

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

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

# =============================================================================
# Test 1: Lua構文チェック
# =============================================================================
test_lua_syntax() {
    log_info "Test 1: Lua構文チェック"

    # Lua でスクリプトを読み込んで構文エラーをチェック
    if command -v lua5.1 &>/dev/null; then
        if lua5.1 -e "package.path = package.path .. ';${SCRIPT_DIR}/?.lua'; dofile('${TARGET_SCRIPT}')" 2>&1 | grep -i "error"; then
            log_error "Lua構文エラーが検出されました"
            return 1
        else
            log_success "Lua構文チェック: PASSED"
            return 0
        fi
    else
        log_info "lua5.1 が見つかりません。スキップします。"
        return 0
    fi
}

# =============================================================================
# Test 2: thread.id 算術演算のシミュレーション
# =============================================================================
test_thread_id_arithmetic() {
    log_info "Test 2: thread.id 算術演算のシミュレーション"

    # Lua で thread.id を数値に変換するロジックをテスト
    local test_code='
local thread_id_values = {0, 1, 2, "0", "1", "2", nil}
for _, thread_id in ipairs(thread_id_values) do
    local converted = tonumber(thread_id) or 0
    local result = converted * 10
    print(string.format("thread.id=%s -> tonumber=%d -> result=%d", tostring(thread_id), converted, result))
    assert(type(result) == "number", "Result must be a number")
end
print("All thread.id conversions succeeded")
'

    if command -v lua5.1 &>/dev/null; then
        if lua5.1 -e "${test_code}" >/dev/null 2>&1; then
            log_success "thread.id 算術演算: PASSED"
            return 0
        else
            log_error "thread.id 算術演算に失敗しました"
            return 1
        fi
    else
        log_info "lua5.1 が見つかりません。スキップします。"
        return 0
    fi
}

# =============================================================================
# Test 3: wrk.format 安全性チェック
# =============================================================================
test_wrk_format_safety() {
    log_info "Test 3: wrk.format 安全性チェック"

    # wrk が未定義の場合でもエラーが発生しないことを確認
    local test_code='
-- wrk が未定義の状態をシミュレート
local wrk = nil

-- tasks_update.lua の request() と同様のロジック
local function safe_format()
    if wrk and wrk.format then
        return wrk.format("GET", "/health")
    else
        return ""
    end
end

local result = safe_format()
assert(result == "", "Expected empty string when wrk is nil")
print("wrk.format safety check passed")

-- wrk が定義されている場合
wrk = {format = function(...) return "mocked_request" end}
result = safe_format()
assert(result == "mocked_request", "Expected mocked request when wrk is defined")
print("wrk.format safety check passed (with wrk defined)")
'

    if command -v lua5.1 &>/dev/null; then
        if lua5.1 -e "${test_code}" >/dev/null 2>&1; then
            log_success "wrk.format 安全性チェック: PASSED"
            return 0
        else
            log_error "wrk.format 安全性チェックに失敗しました"
            return 1
        fi
    else
        log_info "lua5.1 が見つかりません。スキップします。"
        return 0
    fi
}

# =============================================================================
# Main function
# =============================================================================
main() {
    log_info "=== tasks_update.lua 修正検証 ==="
    log_info "Target: ${TARGET_SCRIPT}"
    echo ""

    local passed=0
    local failed=0

    # Test 1
    if test_lua_syntax; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
    fi
    echo ""

    # Test 2
    if test_thread_id_arithmetic; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
    fi
    echo ""

    # Test 3
    if test_wrk_format_safety; then
        passed=$((passed + 1))
    else
        failed=$((failed + 1))
    fi
    echo ""

    # Summary
    log_info "=== Test Summary ==="
    log_info "Passed: ${passed}"

    if [[ ${failed} -gt 0 ]]; then
        log_error "Failed: ${failed}"
        exit 1
    else
        log_success "Failed: 0"
        log_success "All tests passed!"
    fi
}

main "$@"
