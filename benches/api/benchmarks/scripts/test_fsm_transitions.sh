#!/usr/bin/env bash
# Test script for IMPL-TUS3-001+002: FSM transition constraints
# Verifies that Lua VALID_TRANSITIONS matches server-side is_valid_transition

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LUA_SCRIPT="${SCRIPT_DIR}/tasks_update_status.lua"
TEST_IDS_SCRIPT="${SCRIPT_DIR}/test_ids.lua"

echo "=== FSM Transition Matrix Test ==="
echo

# Expected transitions from transaction.rs L239-250
declare -A EXPECTED_TRANSITIONS=(
    ["pending:in_progress"]=1
    ["pending:cancelled"]=1
    ["in_progress:completed"]=1
    ["in_progress:pending"]=1
    ["in_progress:cancelled"]=1
    ["completed:pending"]=1
)

# Invalid transitions (should NOT be in VALID_TRANSITIONS)
declare -A INVALID_TRANSITIONS=(
    ["pending:completed"]=1
    ["pending:pending"]=1
    ["in_progress:in_progress"]=1
    ["completed:completed"]=1
    ["completed:in_progress"]=1
    ["completed:cancelled"]=1
    ["cancelled:pending"]=1
    ["cancelled:in_progress"]=1
    ["cancelled:completed"]=1
    ["cancelled:cancelled"]=1
)

# Extract VALID_TRANSITIONS from Lua script
if ! grep -q "VALID_TRANSITIONS" "$LUA_SCRIPT"; then
    echo "❌ FAIL: VALID_TRANSITIONS table not found in $LUA_SCRIPT"
    exit 1
fi

echo "✓ VALID_TRANSITIONS table exists"

# Check if all expected transitions are present
FAIL_COUNT=0

for transition in "${!EXPECTED_TRANSITIONS[@]}"; do
    from="${transition%%:*}"
    to="${transition##*:}"

    # Pattern: ["from"] = {..., "to", ...}
    if ! grep -A 5 "VALID_TRANSITIONS = {" "$LUA_SCRIPT" | \
         grep -A 1 "\[\"$from\"\]" | \
         grep -q "\"$to\""; then
        echo "❌ FAIL: Expected transition $from -> $to not found in VALID_TRANSITIONS"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    else
        echo "✓ Valid transition: $from -> $to"
    fi
done

# Check that invalid transitions are NOT present
for transition in "${!INVALID_TRANSITIONS[@]}"; do
    from="${transition%%:*}"
    to="${transition##*:}"

    if grep -A 5 "VALID_TRANSITIONS = {" "$LUA_SCRIPT" | \
       grep -A 1 "\[\"$from\"\]" | \
       grep -q "\"$to\""; then
        echo "❌ FAIL: Invalid transition $from -> $to found in VALID_TRANSITIONS"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    else
        echo "✓ Invalid transition correctly absent: $from -> $to"
    fi
done

echo
echo "=== FSM Function Tests ==="
echo

# Check next_valid_status function exists
if ! grep -q "function next_valid_status" "$LUA_SCRIPT"; then
    echo "❌ FAIL: next_valid_status function not found"
    FAIL_COUNT=$((FAIL_COUNT + 1))
else
    echo "✓ next_valid_status function exists"
fi

# Check cancelled returns nil
if ! grep -A 10 "function next_valid_status" "$LUA_SCRIPT" | \
     grep -q "return nil"; then
    echo "❌ FAIL: next_valid_status does not handle nil return (cancelled state)"
    FAIL_COUNT=$((FAIL_COUNT + 1))
else
    echo "✓ next_valid_status handles nil return"
fi

echo
echo "=== test_ids.lua Status Field Test ==="
echo

# Check status field in task_states initialization
if ! grep -A 2 "table.insert(task_states" "$TEST_IDS_SCRIPT" | \
     grep -q "status"; then
    echo "❌ FAIL: status field not found in task_states initialization"
    FAIL_COUNT=$((FAIL_COUNT + 1))
else
    echo "✓ status field exists in task_states"
fi

# Check get_task_state returns status
if ! grep -A 5 "function M.get_task_state" "$TEST_IDS_SCRIPT" | \
     grep -q "status"; then
    echo "❌ FAIL: get_task_state does not return status"
    FAIL_COUNT=$((FAIL_COUNT + 1))
else
    echo "✓ get_task_state returns status"
fi

# Check set_version_and_status function exists
if ! grep -q "function M.set_version_and_status" "$TEST_IDS_SCRIPT"; then
    echo "❌ FAIL: set_version_and_status function not found"
    FAIL_COUNT=$((FAIL_COUNT + 1))
else
    echo "✓ set_version_and_status function exists"
fi

# Check reset_versions resets status
if ! grep -A 3 "function M.reset_versions" "$TEST_IDS_SCRIPT" | \
     grep -q "status"; then
    echo "❌ FAIL: reset_versions does not reset status"
    FAIL_COUNT=$((FAIL_COUNT + 1))
else
    echo "✓ reset_versions resets status"
fi

echo
echo "=== common.lua extract_status Test ==="
echo

COMMON_SCRIPT="${SCRIPT_DIR}/common.lua"

# Check extract_status function exists
if ! grep -q "function M.extract_status" "$COMMON_SCRIPT"; then
    echo "❌ FAIL: extract_status function not found in common.lua"
    FAIL_COUNT=$((FAIL_COUNT + 1))
else
    echo "✓ extract_status function exists"
fi

echo
echo "=== Summary ==="
if [ $FAIL_COUNT -eq 0 ]; then
    echo "✅ All tests passed"
    exit 0
else
    echo "❌ $FAIL_COUNT test(s) failed"
    exit 1
fi
