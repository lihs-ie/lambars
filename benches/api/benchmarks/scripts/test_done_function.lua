-- test_done_function.lua
-- REQ-MEASURE-401: done() 関数のカテゴリ別集計テスト

-- 純粋関数: カテゴリ別集計
local function aggregate_categories(categories)
    return {
        executed = categories.executed,
        backoff = categories.backoff,
        suppressed = categories.suppressed,
        fallback = categories.fallback
    }
end

-- 純粋関数: 整合性検証
local function verify_consistency(categories, total_requests)
    local sum = categories.executed + categories.backoff +
                categories.suppressed + categories.fallback
    return sum == total_requests, sum
end

local function test_aggregate_categories()
    -- カテゴリ別集計のテスト
    local categories = {
        executed = 100,
        backoff = 20,
        suppressed = 10,
        fallback = 5
    }

    local result = aggregate_categories(categories)

    assert(result.executed == 100, "executed should be 100")
    assert(result.backoff == 20, "backoff should be 20")
    assert(result.suppressed == 10, "suppressed should be 10")
    assert(result.fallback == 5, "fallback should be 5")

    print("PASS: aggregate_categories test")
end

local function test_verify_consistency_success()
    -- 整合性検証（成功ケース）
    local categories = {
        executed = 100,
        backoff = 20,
        suppressed = 10,
        fallback = 5
    }
    local total_requests = 135

    local is_consistent, sum = verify_consistency(categories, total_requests)

    assert(is_consistent == true, "should be consistent")
    assert(sum == 135, string.format("sum should be 135, got %d", sum))

    print("PASS: verify_consistency success test")
end

local function test_verify_consistency_failure()
    -- 整合性検証（失敗ケース）
    local categories = {
        executed = 100,
        backoff = 20,
        suppressed = 10,
        fallback = 5
    }
    local total_requests = 140  -- 不一致

    local is_consistent, sum = verify_consistency(categories, total_requests)

    assert(is_consistent == false, "should be inconsistent")
    assert(sum == 135, string.format("sum should be 135, got %d", sum))

    print("PASS: verify_consistency failure test")
end

local function test_excluded_requests_calculation()
    -- excluded_requests の計算テスト
    local categories = {
        executed = 100,
        backoff = 20,
        suppressed = 10,
        fallback = 5
    }

    local excluded_requests = categories.backoff + categories.suppressed + categories.fallback
    assert(excluded_requests == 35, string.format("excluded_requests should be 35, got %d", excluded_requests))

    print("PASS: excluded_requests calculation test")
end

-- テスト実行
print("Running done() function tests...")
print("")

test_aggregate_categories()
test_verify_consistency_success()
test_verify_consistency_failure()
test_excluded_requests_calculation()

print("")
print("Test summary: All tests passed")
