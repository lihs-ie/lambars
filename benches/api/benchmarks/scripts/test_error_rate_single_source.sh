#!/usr/bin/env bash
# Phase B: error_rate 単一ソース化のテストスクリプト
#
# このスクリプトは以下をテストする:
# - LUA-001: result_collector.lua の error_rate 計算が 409 を含むこと
# - LUA-002: error_tracker.lua の error_rate() が total_error_rate() を返すこと
# - LUA-003: merge_lua_metrics.py にコメントが追加されていること

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# カラー出力
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

test_count=0
pass_count=0
fail_count=0

function run_test() {
    local test_name="$1"
    local test_command="$2"

    test_count=$((test_count + 1))
    echo -n "[$test_count] $test_name ... "

    if eval "$test_command" > /dev/null 2>&1; then
        echo -e "${GREEN}PASS${NC}"
        pass_count=$((pass_count + 1))
        return 0
    else
        echo -e "${RED}FAIL${NC}"
        fail_count=$((fail_count + 1))
        return 1
    fi
}

function run_test_with_output() {
    local test_name="$1"
    local test_command="$2"
    local expected_pattern="$3"

    test_count=$((test_count + 1))
    echo -n "[$test_count] $test_name ... "

    output=$(eval "$test_command" 2>&1 || true)
    if echo "$output" | grep -q "$expected_pattern"; then
        echo -e "${GREEN}PASS${NC}"
        pass_count=$((pass_count + 1))
        return 0
    else
        echo -e "${RED}FAIL${NC}"
        echo "  Expected pattern: $expected_pattern"
        echo "  Actual output: $output"
        fail_count=$((fail_count + 1))
        return 1
    fi
}

echo "========================================="
echo "Phase B: error_rate 単一ソース化テスト"
echo "========================================="
echo ""

# ========================================
# LUA-001: result_collector.lua のテスト
# ========================================
echo "[LUA-001] result_collector.lua の error_rate 計算"

# Test 1: error_rate 計算が total_http_errors を使用していること
run_test_with_output \
    "error_rate が total_http_errors / tracked_requests で計算されている" \
    "grep -A 2 'local total_http_errors' result_collector.lua | grep 'M.results.error_rate = total_http_errors / tracked_requests'" \
    "M.results.error_rate = total_http_errors / tracked_requests"

# Test 2: non_conflict_errors 変数が削除されていること
if grep -q 'local non_conflict_errors' result_collector.lua 2>/dev/null; then
    test_count=$((test_count + 1))
    echo -e "[$test_count] non_conflict_errors 変数が削除されている ... ${RED}FAIL${NC}"
    fail_count=$((fail_count + 1))
else
    test_count=$((test_count + 1))
    echo -e "[$test_count] non_conflict_errors 変数が削除されている ... ${GREEN}PASS${NC}"
    pass_count=$((pass_count + 1))
fi

# Test 3: ラベルが "Error rate:" になっていること
run_test_with_output \
    "出力ラベルが 'Error rate:' になっている" \
    "grep 'format_rate.*Error rate' result_collector.lua" \
    'format_rate("Error rate:", M.results.error_rate)'

# ========================================
# LUA-002: error_tracker.lua のテスト
# ========================================
echo ""
echo "[LUA-002] error_tracker.lua の error_rate() メソッド"

# Test 4: error_rate() が total_error_rate() を返すこと
run_test_with_output \
    "error_rate() が total_error_rate() を呼び出している" \
    "grep 'function M.error_rate()' error_tracker.lua -A 1" \
    "M.total_error_rate()"

# Test 5: get_summary() の error_rate フィールドが total_error_rate() を使用すること
run_test_with_output \
    "get_summary() の error_rate が total_error_rate() を使用している" \
    "grep 'error_rate = M' error_tracker.lua | grep 'total_error_rate'" \
    "error_rate = M.total_error_rate()"

# ========================================
# LUA-003: merge_lua_metrics.py のテスト
# ========================================
echo ""
echo "[LUA-003] merge_lua_metrics.py のコメント"

# Test 6: error_rate 計算にコメントが追加されていること
run_test_with_output \
    "error_rate 計算にコメントが追加されている" \
    "grep -B 3 'error_rate = total_errors' merge_lua_metrics.py" \
    "socket_errors"

# ========================================
# 構文チェック
# ========================================
echo ""
echo "[構文チェック]"

# Test 7: result_collector.lua の構文チェック
run_test \
    "result_collector.lua の構文チェック" \
    "luac -p result_collector.lua"

# Test 8: error_tracker.lua の構文チェック
run_test \
    "error_tracker.lua の構文チェック" \
    "luac -p error_tracker.lua"

# Test 9: merge_lua_metrics.py の構文チェック
run_test \
    "merge_lua_metrics.py の構文チェック" \
    "python3 -m py_compile merge_lua_metrics.py"

# ========================================
# 結果サマリー
# ========================================
echo ""
echo "========================================="
echo "テスト結果"
echo "========================================="
echo "Total: $test_count"
echo -e "${GREEN}Pass:  $pass_count${NC}"
echo -e "${RED}Fail:  $fail_count${NC}"
echo ""

if [ $fail_count -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed.${NC}"
    exit 1
fi
