#!/usr/bin/env bash
# test_extract_error_code.sh - Test common.extract_error_code function

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

# Test extract_error_code with VERSION_CONFLICT
luajit <<'EOF'
package.path = package.path .. ";./?.lua"
local common = require("common")

local test_body_1 = '{"code":"VERSION_CONFLICT","message":"Version mismatch"}'
local result_1 = common.extract_error_code(test_body_1)
assert(result_1 == "VERSION_CONFLICT", "Expected VERSION_CONFLICT, got " .. tostring(result_1))

local test_body_2 = '{"code":"OTHER_ERROR","message":"Other error"}'
local result_2 = common.extract_error_code(test_body_2)
assert(result_2 == "OTHER_ERROR", "Expected OTHER_ERROR, got " .. tostring(result_2))

local test_body_3 = '{"message":"No code field"}'
local result_3 = common.extract_error_code(test_body_3)
assert(result_3 == nil, "Expected nil, got " .. tostring(result_3))

local result_4 = common.extract_error_code("")
assert(result_4 == nil, "Expected nil for empty string, got " .. tostring(result_4))

local result_5 = common.extract_error_code(nil)
assert(result_5 == nil, "Expected nil for nil input, got " .. tostring(result_5))

print("PASS: common.extract_error_code tests")
EOF

echo "All tests passed"
