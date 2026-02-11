#!/usr/bin/env bash
# Test script for IMPL-TU2-002: Verify tasks_update_status.lua exists and is correct

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LUA_SCRIPT="${SCRIPT_DIR}/tasks_update_status.lua"
YAML_SCENARIO="${SCRIPT_DIR}/../scenarios/tasks_update_status.yaml"
RUN_BENCHMARK="${SCRIPT_DIR}/../run_benchmark.sh"
THRESHOLDS="${SCRIPT_DIR}/../thresholds.yaml"

echo "=== Test: tasks_update_status files exist and are correct ==="

# Test 1: Check tasks_update_status.lua exists
echo "Test 1: tasks_update_status.lua should exist"
if [[ ! -f "${LUA_SCRIPT}" ]]; then
    echo "FAIL: tasks_update_status.lua does not exist"
    exit 1
else
    echo "PASS: tasks_update_status.lua exists"
fi

# Test 2: Check it uses PATCH method
echo "Test 2: Should use PATCH method"
if ! grep -q 'wrk.format.*"PATCH"' "${LUA_SCRIPT}"; then
    echo "FAIL: PATCH method not found"
    exit 1
else
    echo "PASS: PATCH method found"
fi

# Test 3: Check endpoint is /tasks/{id}/status
echo "Test 3: Should target /tasks/{id}/status endpoint"
if ! grep -q '/tasks/.*status' "${LUA_SCRIPT}"; then
    echo "FAIL: /tasks/{id}/status endpoint not found"
    exit 1
else
    echo "PASS: /tasks/{id}/status endpoint found"
fi

# Test 4: Check tasks_update_status.yaml exists
echo "Test 4: tasks_update_status.yaml should exist"
if [[ ! -f "${YAML_SCENARIO}" ]]; then
    echo "FAIL: tasks_update_status.yaml does not exist"
    exit 1
else
    echo "PASS: tasks_update_status.yaml exists"
fi

# Test 5: Check run_benchmark.sh has mapping
echo "Test 5: run_benchmark.sh should have /tasks/{id}/status mapping"
if ! grep -q 'tasks_update_status' "${RUN_BENCHMARK}"; then
    echo "FAIL: tasks_update_status mapping not found in run_benchmark.sh"
    exit 1
else
    echo "PASS: tasks_update_status mapping found"
fi

# Test 6: Check thresholds.yaml has tasks_update_status section
echo "Test 6: thresholds.yaml should have tasks_update_status section"
if ! grep -q 'tasks_update_status:' "${THRESHOLDS}"; then
    echo "FAIL: tasks_update_status section not found in thresholds.yaml"
    exit 1
else
    echo "PASS: tasks_update_status section found"
fi

# Test 7: Lua syntax check
echo "Test 7: Lua syntax check"
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
