#!/usr/bin/env lua
-- Test: error_tracker の二重初期化でスレッドリストが消去されないことを検証

local error_tracker = require("benches/api/benchmarks/scripts/error_tracker")

-- Mock thread object
local function create_mock_thread()
    local storage = {}
    return {
        set = function(self, key, value) storage[key] = value end,
        get = function(self, key) return storage[key] end,
    }
end

-- Test 1: 二重初期化でスレッドリストがリセットされないこと
print("[TEST] error_tracker double init should not reset threads")

-- 初回 init
error_tracker.init()
assert(#error_tracker.threads == 0, "threads should be empty initially")

-- スレッド登録
local thread1 = create_mock_thread()
local thread2 = create_mock_thread()
error_tracker.setup_thread(thread1)
error_tracker.setup_thread(thread2)
assert(#error_tracker.threads == 2, "threads should have 2 entries after setup_thread")

-- 二重初期化（result_collector.init() から呼ばれる想定）
error_tracker.init()

-- 期待: スレッドリストは保持されている
if #error_tracker.threads == 2 then
    print("✅ PASS: threads preserved after double init (expected: 2, got: " .. #error_tracker.threads .. ")")
    os.exit(0)
else
    print("❌ FAIL: threads were reset (expected: 2, got: " .. #error_tracker.threads .. ")")
    os.exit(1)
end
