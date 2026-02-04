#!/usr/bin/env bash
# Test script for error rate improvement modifications

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'
TESTS_PASSED=0
TESTS_FAILED=0

print_test_header() { echo -e "\n========================================\n$1\n========================================"; }
print_success() { echo -e "${GREEN}✓ $1${NC}"; ((TESTS_PASSED++)); }
print_failure() { echo -e "${RED}✗ $1${NC}"; ((TESTS_FAILED++)); }
print_info() { echo -e "${YELLOW}ℹ $1${NC}"; }

check_lua_installation() {
    print_test_header "Checking Lua Installation"
    if command -v luajit &> /dev/null; then
        LUA_CMD="luajit"
        print_success "LuaJIT is installed: $(luajit -v 2>&1 | head -n1)"
        return 0
    elif command -v lua &> /dev/null; then
        LUA_CMD="lua"
        print_success "Lua is installed: $(lua -v 2>&1 | head -n1)"
        return 0
    else
        print_failure "Lua/LuaJIT is not installed"
        return 1
    fi
}

check_file_pattern() {
    local file=$1 pattern=$2 success_msg=$3 failure_msg=$4
    if [ ! -f "$file" ]; then
        print_failure "$(basename "$file") not found"
        return 1
    fi
    if grep -q "$pattern" "$file"; then
        print_success "$success_msg"
        return 0
    else
        print_failure "$failure_msg"
        return 1
    fi
}

test_batch_size_modification() {
    print_test_header "REQ-ERROR-001: tasks_bulk Batch Size Modification"
    check_file_pattern "$SCRIPT_DIR/tasks_bulk.lua" "batch_sizes = {10, 50, 100}" \
        "Batch sizes correctly set to {10, 50, 100}" "Batch sizes not correctly set"
    check_file_pattern "$SCRIPT_DIR/tasks_bulk.lua" "BULK_LIMIT" \
        "Comment references BULK_LIMIT" "Comment does not reference BULK_LIMIT"
}

test_http_status_tracking() {
    print_test_header "REQ-ERROR-002: HTTP Status Tracking Implementation"
    check_file_pattern "$SCRIPT_DIR/error_tracker.lua" "function M.setup_thread" \
        "error_tracker.lua: setup_thread() implemented" "error_tracker.lua: setup_thread() not found"
    check_file_pattern "$SCRIPT_DIR/error_tracker.lua" "function M.track_thread_response" \
        "error_tracker.lua: track_thread_response() implemented" "error_tracker.lua: track_thread_response() not found"
    check_file_pattern "$SCRIPT_DIR/error_tracker.lua" "function M.get_thread_aggregated_summary" \
        "error_tracker.lua: get_thread_aggregated_summary() implemented" "error_tracker.lua: get_thread_aggregated_summary() not found"
    check_file_pattern "$SCRIPT_DIR/error_tracker.lua" "M.threads = {}" \
        "error_tracker.lua: thread list initialized" "error_tracker.lua: thread list not initialized"
    check_file_pattern "$SCRIPT_DIR/common.lua" "function M.create_threaded_handlers" \
        "common.lua: create_threaded_handlers() implemented" "common.lua: create_threaded_handlers() not found"
    check_file_pattern "$SCRIPT_DIR/tasks_bulk.lua" "create_threaded_handlers" \
        "tasks_bulk.lua: uses threaded handlers" "tasks_bulk.lua: does not use threaded handlers"
    check_file_pattern "$SCRIPT_DIR/tasks_bulk.lua" "setup = handlers.setup" \
        "tasks_bulk.lua: setup handler assigned" "tasks_bulk.lua: setup handler not assigned"
    check_file_pattern "$SCRIPT_DIR/tasks_update.lua" "function setup(thread)" \
        "tasks_update.lua: setup() function defined" "tasks_update.lua: setup() function not found"
    check_file_pattern "$SCRIPT_DIR/tasks_update.lua" "error_tracker.setup_thread(thread)" \
        "tasks_update.lua: calls error_tracker.setup_thread()" "tasks_update.lua: does not call error_tracker.setup_thread()"
    check_file_pattern "$SCRIPT_DIR/tasks_update.lua" "error_tracker.track_thread_response(status)" \
        "tasks_update.lua: tracks thread responses" "tasks_update.lua: does not track thread responses"
    check_file_pattern "$SCRIPT_DIR/tasks_update.lua" "error_tracker.get_thread_aggregated_summary()" \
        "tasks_update.lua: prints status distribution" "tasks_update.lua: does not print status distribution"
}

test_id_pool_improvements() {
    print_test_header "REQ-ERROR-003: tasks_update ID Pool Improvements"
    local update_file="$SCRIPT_DIR/tasks_update.lua"
    if [ ! -f "$update_file" ]; then
        print_failure "tasks_update.lua not found"
        return
    fi

    if grep -q "WRK_THREADS" "$update_file" && grep -q "ID_POOL_SIZE" "$update_file"; then
        print_success "tasks_update.lua: reads WRK_THREADS and ID_POOL_SIZE"
    else
        print_failure "tasks_update.lua: does not read WRK_THREADS or ID_POOL_SIZE"
    fi

    if grep -q "id_start" "$update_file" && grep -q "id_end" "$update_file" && grep -q "id_range" "$update_file"; then
        print_success "tasks_update.lua: implements ID range partitioning"
    else
        print_failure "tasks_update.lua: does not implement ID range partitioning"
    fi

    if grep -q "thread:set(\"id_start\"" "$update_file" && \
       grep -q "thread:set(\"id_end\"" "$update_file" && \
       grep -q "thread:set(\"id_range\"" "$update_file"; then
        print_success "tasks_update.lua: stores ID range in thread"
    else
        print_failure "tasks_update.lua: does not store ID range in thread"
    fi

    if grep -q "local_index" "$update_file" && grep -q "global_index" "$update_file"; then
        print_success "tasks_update.lua: uses thread-specific ID selection"
    else
        print_failure "tasks_update.lua: does not use thread-specific ID selection"
    fi

    if grep -q "wrk.thread:get(\"id_start\")" "$update_file" && grep -q "wrk.thread:get(\"id_range\")" "$update_file"; then
        print_success "tasks_update.lua: retrieves ID range from thread"
    else
        print_failure "tasks_update.lua: does not retrieve ID range from thread"
    fi

    check_file_pattern "$SCRIPT_DIR/test_ids.lua" "ID_POOL_SIZE" \
        "test_ids.lua: supports ID_POOL_SIZE environment variable" "test_ids.lua: does not support ID_POOL_SIZE"

    check_file_pattern "$SCRIPT_DIR/test_ids.lua" "SEED" \
        "test_ids.lua: supports SEED environment variable" "test_ids.lua: does not support SEED"

    check_file_pattern "$SCRIPT_DIR/test_ids.lua" "id_rng_state" \
        "test_ids.lua: uses independent RNG for ID generation" "test_ids.lua: does not use independent RNG"

    if grep -q "RETRY_COUNT.*or 0" "$update_file"; then
        print_success "tasks_update.lua: RETRY_COUNT defaults to 0 (Phase 1: no retry)"
    else
        print_failure "tasks_update.lua: RETRY_COUNT should default to 0 for Phase 1"
    fi

    if grep -q "ID_POOL_SIZE.*<.*WRK_THREADS" "$update_file"; then
        print_success "tasks_update.lua: validates ID_POOL_SIZE >= WRK_THREADS"
    else
        print_failure "tasks_update.lua: missing validation for ID_POOL_SIZE >= WRK_THREADS"
    fi

    if grep -q "WRK_THREADS.*is required" "$update_file"; then
        print_success "tasks_update.lua: requires WRK_THREADS environment variable"
    else
        print_failure "tasks_update.lua: missing WRK_THREADS requirement check"
    fi

    # Check scenario_env.sh exports ID_POOL_SIZE and WRK_THREADS
    local scenario_env_file="$SCRIPT_DIR/scenario_env.sh"
    if [ -f "$scenario_env_file" ]; then
        if grep -q "ID_POOL_SIZE" "$scenario_env_file" && grep -q "WRK_THREADS" "$scenario_env_file"; then
            print_success "scenario_env.sh: exports ID_POOL_SIZE and WRK_THREADS"
        else
            print_failure "scenario_env.sh: does not export ID_POOL_SIZE or WRK_THREADS"
        fi
    else
        print_info "scenario_env.sh not found (optional check)"
    fi
}

test_lua_syntax() {
    print_test_header "Testing Lua Syntax"
    local files=("tasks_bulk.lua" "tasks_update.lua" "test_ids.lua" "error_tracker.lua" "common.lua")
    for file in "${files[@]}"; do
        local filepath="$SCRIPT_DIR/$file"
        if [ ! -f "$filepath" ]; then
            print_failure "File not found: $file"
            continue
        fi
        if $LUA_CMD -e "dofile('$filepath')" 2>&1 | grep -q "error"; then
            print_failure "Syntax error in: $file"
            $LUA_CMD -e "dofile('$filepath')" 2>&1 | head -n 10
        else
            print_success "Syntax OK: $file"
        fi
    done
}

test_error_tracker_functionality() {
    print_test_header "Testing error_tracker Module Functionality"
    local result
    result=$($LUA_CMD -e "package.path = package.path .. ';$SCRIPT_DIR/?.lua'; \
        local et = require('error_tracker'); print(type(et.threads))" 2>&1)
    [ "$result" = "table" ] && print_success "error_tracker.threads is a table" || \
        print_failure "error_tracker.threads is not a table (got: $result)"

    result=$($LUA_CMD -e "package.path = package.path .. ';$SCRIPT_DIR/?.lua'; \
        local et = require('error_tracker'); \
        local mock_thread = {set = function(self, key, value) self[key] = value end, \
                             get = function(self, key) return self[key] end}; \
        et.setup_thread(mock_thread); print(#et.threads)" 2>&1)
    [ "$result" = "1" ] && print_success "setup_thread() adds thread to list" || \
        print_failure "setup_thread() does not add thread to list (got: $result)"
}

test_wrk_threads_validation() {
    print_test_header "Testing WRK_THREADS Validation (Runtime)"
    local test_script="$SCRIPT_DIR/tasks_update.lua"
    if [ ! -f "$test_script" ]; then
        print_failure "tasks_update.lua not found"
        return
    fi

    # Test: WRK_THREADS unset should exit with error
    local test_result
    test_result=$($LUA_CMD -e "
        package.path = package.path .. ';$SCRIPT_DIR/?.lua'
        wrk = {thread = {id = 0, get = function() return nil end, set = function() end}}
        os.getenv = function(key) if key == 'ID_POOL_SIZE' then return '10' else return nil end end
        os.exit = function(code) error('EXIT_' .. code) end
        dofile('$test_script')
        local ok, err = pcall(setup, wrk.thread)
        if err and err:match('EXIT_1') then print('EXIT_WITH_ERROR') else print('NO_EXIT') end
    " 2>&1)

    if echo "$test_result" | grep -q "EXIT_WITH_ERROR"; then
        print_success "tasks_update.lua exits when WRK_THREADS is unset"
    else
        print_failure "tasks_update.lua does not exit when WRK_THREADS is unset (got: $test_result)"
    fi

    # Test: WRK_THREADS=0 should exit with error
    test_result=$($LUA_CMD -e "
        package.path = package.path .. ';$SCRIPT_DIR/?.lua'
        wrk = {thread = {id = 0, get = function() return nil end, set = function() end}}
        os.getenv = function(key) if key == 'WRK_THREADS' then return '0' elseif key == 'ID_POOL_SIZE' then return '10' else return nil end end
        os.exit = function(code) error('EXIT_' .. code) end
        dofile('$test_script')
        local ok, err = pcall(setup, wrk.thread)
        if err and err:match('EXIT_1') then print('EXIT_WITH_ERROR') else print('NO_EXIT') end
    " 2>&1)

    if echo "$test_result" | grep -q "EXIT_WITH_ERROR"; then
        print_success "tasks_update.lua exits when WRK_THREADS=0"
    else
        print_failure "tasks_update.lua does not exit when WRK_THREADS=0 (got: $test_result)"
    fi
}

main() {
    echo "Starting Error Rate Improvement Test Suite"
    echo "==========================================="
    echo "Testing REQ-ERROR-001, REQ-ERROR-002, REQ-ERROR-003"
    echo ""

    check_lua_installation || { echo -e "\nERROR: Lua is required to run these tests"; exit 1; }

    test_batch_size_modification
    test_http_status_tracking
    test_id_pool_improvements
    test_lua_syntax
    test_error_tracker_functionality
    test_wrk_threads_validation

    echo -e "\n========================================\nTest Summary\n========================================"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}\n${RED}Failed: $TESTS_FAILED${NC}\n"

    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}All tests passed!${NC}\n"
        echo "Next steps:"
        echo "  1. Run benchmark: wrk -t4 -c10 -d30s -s $SCRIPT_DIR/tasks_bulk.lua http://localhost:3002"
        echo "  2. Run benchmark: ID_POOL_SIZE=1000 WRK_THREADS=4 wrk -t4 -c10 -d30s -s $SCRIPT_DIR/tasks_update.lua http://localhost:3002"
        echo "  3. Verify error rate is below 10%"
        echo "  4. Check HTTP status distribution in output"
        exit 0
    else
        echo -e "${RED}Some tests failed!${NC}"
        exit 1
    fi
}

main "$@"
