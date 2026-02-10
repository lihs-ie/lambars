#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== FSM Transition Matrix Validation Test ==="
echo ""
echo "[Test 1] Validating VALID_TRANSITIONS structure..."

luajit <<'LUAEOF'
package.path = package.path .. ';./?.lua'
local file = io.open('tasks_update_status.lua', 'r')
local content = file:read('*all')
file:close()

local EXPECTED_TRANSITIONS = {
    ['pending'] = {'in_progress', 'cancelled'},
    ['in_progress'] = {'completed', 'pending', 'cancelled'},
    ['completed'] = {'pending'},
    ['cancelled'] = {}
}

for from_state, expected_to in pairs(EXPECTED_TRANSITIONS) do
    local pattern = '%["' .. from_state .. '"%]%s*=%s*{([^}]*)}'
    local match = content:match(pattern)
    if not match then error('State ' .. from_state .. ' not found') end

    local count = 0
    for _ in match:gmatch('"[^"]+"') do count = count + 1 end

    if count ~= #expected_to then
        error('State ' .. from_state .. ' has ' .. count .. ' transitions, expected ' .. #expected_to)
    end
    print('  ✓ State "' .. from_state .. '" has ' .. count .. ' valid transitions')
end

print('  PASS: All transition states are correctly defined')
LUAEOF

echo ""
echo "[Test 2] Validating test_ids.lua status field..."

luajit <<'LUAEOF'
package.path = package.path .. ';./?.lua'
local test_ids = require('test_ids')

local state = assert(test_ids.get_task_state(1))
assert(state.status == 'pending', 'Initial status is not pending')
print('  ✓ get_task_state returns status field (initial: ' .. state.status .. ')')

assert(test_ids.set_version_and_status(1, 2, 'in_progress'))
local updated_state = assert(test_ids.get_task_state(1))
assert(updated_state.version == 2, 'Version not updated')
assert(updated_state.status == 'in_progress', 'Status not updated')
print('  ✓ set_version_and_status correctly updates version and status')

test_ids.reset_versions()
local reset_state = assert(test_ids.get_task_state(1))
assert(reset_state.version == 1, 'Version not reset')
assert(reset_state.status == 'pending', 'Status not reset')
print('  ✓ reset_versions correctly resets status to pending')

print('  PASS: test_ids.lua status management works correctly')
LUAEOF

echo ""
echo "[Test 3] Validating common.extract_status..."

luajit <<'LUAEOF'
package.path = package.path .. ';./?.lua'
local common = require('common')

assert(common.extract_status('{"status": "in_progress", "version": 2}') == 'in_progress')
print('  ✓ Extracts status from JSON')

assert(common.extract_status('{"version": 2}') == nil)
print('  ✓ Returns nil for missing status')

assert(common.extract_status('') == nil)
print('  ✓ Returns nil for empty string')

assert(common.extract_status(nil) == nil)
print('  ✓ Returns nil for nil input')

print('  PASS: common.extract_status works correctly')
LUAEOF

echo ""
echo "[Test 4] Validating FSM integration in tasks_update_status.lua..."

luajit <<'LUAEOF'
package.path = package.path .. ';./?.lua'
local file = io.open('tasks_update_status.lua', 'r')
local content = file:read('*all')
file:close()

local checks = {
    {'next_valid_status function', 'local function next_valid_status'},
    {'generate_update_body signature', 'generate_update_body%(current_status'},
    {'last_request_status variable', 'last_request_status'},
    {'common.extract_status call', 'common%.extract_status'},
    {'retry_sent_status variable', 'retry_sent_status'},
    {'full jitter backoff', 'math%.random%(0'}
}

for _, check in ipairs(checks) do
    assert(content:match(check[2]), check[1] .. ' not found')
    print('  ✓ ' .. check[1] .. ' exists')
end

print('  PASS: FSM integration in tasks_update_status.lua is correct')
LUAEOF

echo ""
echo "=== All Tests Passed ==="
