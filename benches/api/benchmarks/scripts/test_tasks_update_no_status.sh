#!/usr/bin/env bash
# Test script for IMPL-TU2-001: Verify tasks_update.lua does not include status in PUT payload

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LUA_SCRIPT="${SCRIPT_DIR}/tasks_update.lua"

echo "=== Test: tasks_update.lua does not include status field ==="

# Test 1: Check update_types does not contain "status"
echo "Test 1: update_types array should not contain 'status'"
if grep -q 'update_types.*=.*{.*"status"' "${LUA_SCRIPT}"; then
    echo "FAIL: update_types contains 'status'"
    exit 1
else
    echo "PASS: update_types does not contain 'status'"
fi

# Test 2: Check that there is no "elseif update_type == 'status'" branch
echo "Test 2: generate_update_body should not have status branch"
if grep -q 'elseif update_type == "status"' "${LUA_SCRIPT}"; then
    echo "FAIL: status branch exists in generate_update_body"
    exit 1
else
    echo "PASS: status branch does not exist"
fi

# Test 3: Check that full update does not include status field
echo "Test 3: full update should not include status field"
# Implementation uses "else" branch with payload_table for full update
# Extract only the table literal (between '= {' and '}'), excluding assert lines
# Check for 'status =' assignment within the table definition
if sed -n '/local payload_table = {/,/^        }/p' "${LUA_SCRIPT}" | grep -E '^\s+status\s*=' | grep -v 'payload_table.status'; then
    echo "FAIL: full update includes status field in table definition"
    exit 1
else
    echo "PASS: full update does not include status field"
fi

# Test 4: Lua syntax check
echo "Test 4: Lua syntax check"
if command -v luac >/dev/null 2>&1; then
    if luac -p "${LUA_SCRIPT}" >/dev/null 2>&1; then
        echo "PASS: Lua syntax is valid"
    else
        echo "FAIL: Lua syntax error"
        exit 1
    fi
else
    echo "SKIP: luac not found, skipping syntax check"
fi

echo ""
echo "=== All tests passed ==="
