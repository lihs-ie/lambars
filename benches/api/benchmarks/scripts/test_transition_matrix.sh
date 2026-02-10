#!/bin/bash
# Test script for FSM transition matrix validation (IMPL-TUS3-001)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== FSM Transition Matrix Validation Test ==="

# Test 1: Transition matrix completeness
echo ""
echo "[Test 1] Validating VALID_TRANSITIONS structure..."

luajit <<'LUAEOF'
package.path = package.path .. ';./?.lua'

-- Extract VALID_TRANSITIONS from tasks_update_status.lua
local file = io.open('tasks_update_status.lua', 'r')
local content = file:read('*all')
file:close()

-- Expected transitions from transaction.rs L239-250
local EXPECTED_TRANSITIONS = {
    ['pending'] = {'in_progress', 'cancelled'},
    ['in_progress'] = {'completed', 'pending', 'cancelled'},
    ['completed'] = {'pending'},
    ['cancelled'] = {}
}

-- Verify each state
for from_state, expected_to in pairs(EXPECTED_TRANSITIONS) do
    local pattern = '%["' .. from_state .. '"%]%s*=%s*{([^}]*)}'
    local match = content:match(pattern)
    if not match then
        error('FAIL: State ' .. from_state .. ' not found in VALID_TRANSITIONS')
    end

    -- Count transitions
    local count = 0
    for _ in match:gmatch('"[^"]+"') do
        count = count + 1
    end

    if count ~= #expected_to then
        error('FAIL: State ' .. from_state .. ' has ' .. count .. ' transitions, expected ' .. #expected_to)
    end

    print('  ✓ State "' .. from_state .. '" has ' .. count .. ' valid transitions')
end

print('  PASS: All transition states are correctly defined')
LUAEOF

# Test 2: test_ids.lua status field existence
echo ""
echo "[Test 2] Validating test_ids.lua status field..."

luajit <<'LUAEOF'
package.path = package.path .. ';./?.lua'
local test_ids = require('test_ids')

-- Test get_task_state returns status
local state, err = test_ids.get_task_state(1)
assert(state, 'FAIL: get_task_state returned nil: ' .. (err or 'unknown'))
assert(state.status, 'FAIL: get_task_state does not return status field')
assert(state.status == 'pending', 'FAIL: Initial status is not pending: ' .. tostring(state.status))
print('  ✓ get_task_state returns status field (initial: ' .. state.status .. ')')

-- Test set_version_and_status
local success, set_err = test_ids.set_version_and_status(1, 2, 'in_progress')
assert(success, 'FAIL: set_version_and_status failed: ' .. (set_err or 'unknown'))
local updated_state, get_err = test_ids.get_task_state(1)
assert(updated_state, 'FAIL: get_task_state after set failed: ' .. (get_err or 'unknown'))
assert(updated_state.version == 2, 'FAIL: Version not updated')
assert(updated_state.status == 'in_progress', 'FAIL: Status not updated to in_progress: ' .. tostring(updated_state.status))
print('  ✓ set_version_and_status correctly updates version and status')

-- Test reset_versions
test_ids.reset_versions()
local reset_state, reset_err = test_ids.get_task_state(1)
assert(reset_state, 'FAIL: get_task_state after reset failed: ' .. (reset_err or 'unknown'))
assert(reset_state.version == 1, 'FAIL: Version not reset to 1')
assert(reset_state.status == 'pending', 'FAIL: Status not reset to pending: ' .. tostring(reset_state.status))
print('  ✓ reset_versions correctly resets status to pending')

print('  PASS: test_ids.lua status management works correctly')
LUAEOF

# Test 3: common.extract_status function
echo ""
echo "[Test 3] Validating common.extract_status..."

luajit <<'LUAEOF'
package.path = package.path .. ';./?.lua'
local common = require('common')

-- Test extract_status with valid JSON
local body1 = '{"status": "in_progress", "version": 2}'
local status1 = common.extract_status(body1)
assert(status1 == 'in_progress', 'FAIL: Expected in_progress, got ' .. tostring(status1))
print('  ✓ Extracts status from JSON: ' .. status1)

-- Test extract_status with no status
local body2 = '{"version": 2}'
local status2 = common.extract_status(body2)
assert(status2 == nil, 'FAIL: Expected nil for missing status, got ' .. tostring(status2))
print('  ✓ Returns nil for missing status')

-- Test extract_status with empty string
local status3 = common.extract_status('')
assert(status3 == nil, 'FAIL: Expected nil for empty string, got ' .. tostring(status3))
print('  ✓ Returns nil for empty string')

-- Test extract_status with non-string
local status4 = common.extract_status(nil)
assert(status4 == nil, 'FAIL: Expected nil for nil input, got ' .. tostring(status4))
print('  ✓ Returns nil for nil input')

print('  PASS: common.extract_status works correctly')
LUAEOF

# Test 4: tasks_update_status.lua FSM integration
echo ""
echo "[Test 4] Validating FSM integration in tasks_update_status.lua..."

luajit <<'LUAEOF'
package.path = package.path .. ';./?.lua'

-- Load and inspect tasks_update_status.lua
local file = io.open('tasks_update_status.lua', 'r')
local content = file:read('*all')
file:close()

-- Check for next_valid_status function
assert(content:match('local function next_valid_status'), 'FAIL: next_valid_status function not found')
print('  ✓ next_valid_status function exists')

-- Check for generate_update_body signature change
assert(content:match('generate_update_body%(current_status'), 'FAIL: generate_update_body does not accept current_status parameter')
print('  ✓ generate_update_body accepts current_status parameter')

-- Check for status tracking in response()
assert(content:match('last_request_status'), 'FAIL: last_request_status variable not found')
print('  ✓ last_request_status variable exists')

-- Check for status synchronization in retry_get
assert(content:match('common%.extract_status'), 'FAIL: common.extract_status not called in retry_get')
print('  ✓ common.extract_status is called in retry flow')

-- Check for retry_sent_status
assert(content:match('retry_sent_status'), 'FAIL: retry_sent_status variable not found')
print('  ✓ retry_sent_status variable exists')

-- Check for full jitter backoff
assert(content:match('math%.random%(0'), 'FAIL: Full jitter backoff not implemented')
print('  ✓ Full jitter backoff is implemented')

print('  PASS: FSM integration in tasks_update_status.lua is correct')
LUAEOF

echo ""
echo "=== All Tests Passed ==="
