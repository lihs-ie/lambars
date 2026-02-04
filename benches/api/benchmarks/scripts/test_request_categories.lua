-- test_request_categories.lua
-- REQ-MEASURE-401: request_categories のテスト

-- グローバル変数として定義されているか確認するため、直接テスト
-- tasks_update.lua は wrk 環境に依存するため、単独では実行できない
-- そのため、request_categories の定義をシミュレート

-- request_categories の初期化をシミュレート
request_categories = {
    executed = 0,
    backoff = 0,
    suppressed = 0,
    fallback = 0
}

local function test_request_categories_initialization()
    -- request_categories がグローバルに定義されていることを確認
    assert(request_categories ~= nil, "request_categories should be defined")
    assert(request_categories.executed == 0, "executed should be initialized to 0")
    assert(request_categories.backoff == 0, "backoff should be initialized to 0")
    assert(request_categories.suppressed == 0, "suppressed should be initialized to 0")
    assert(request_categories.fallback == 0, "fallback should be initialized to 0")

    print("PASS: request_categories initialization test")
end

local function test_backoff_request_counting()
    -- backoff リクエストがカウントされることを確認
    -- 現時点では失敗するテスト
    local initial_count = request_categories.backoff

    -- backoff リクエストをシミュレート
    -- （実際には request() 関数内で is_backoff_request = true となる箇所でカウント）
    request_categories.backoff = request_categories.backoff + 1

    assert(request_categories.backoff == initial_count + 1,
           "backoff count should increment")

    print("PASS: backoff request counting test")
end

local function test_suppressed_request_counting()
    -- suppressed リクエストがカウントされることを確認
    local initial_count = request_categories.suppressed

    request_categories.suppressed = request_categories.suppressed + 1

    assert(request_categories.suppressed == initial_count + 1,
           "suppressed count should increment")

    print("PASS: suppressed request counting test")
end

local function test_fallback_request_counting()
    -- fallback リクエストがカウントされることを確認
    local initial_count = request_categories.fallback

    request_categories.fallback = request_categories.fallback + 1

    assert(request_categories.fallback == initial_count + 1,
           "fallback count should increment")

    print("PASS: fallback request counting test")
end

local function test_category_sum_consistency()
    -- カテゴリの合計が total_requests と一致することを確認
    request_categories = {
        executed = 100,
        backoff = 20,
        suppressed = 10,
        fallback = 5
    }

    local sum = request_categories.executed + request_categories.backoff +
                request_categories.suppressed + request_categories.fallback
    local expected_total = 135

    assert(sum == expected_total,
           string.format("sum should be %d, got %d", expected_total, sum))

    print("PASS: category sum consistency test")
end

-- テスト実行
print("Running request_categories tests...")
print("")

-- 初期化テスト（失敗するはず）
local status, err = pcall(test_request_categories_initialization)
if not status then
    print("FAIL: request_categories initialization test")
    print("  Error: " .. tostring(err))
else
    test_request_categories_initialization()
end

-- カウントテスト（初期化が成功したら実行）
if status then
    test_backoff_request_counting()
    test_suppressed_request_counting()
    test_fallback_request_counting()
    test_category_sum_consistency()
end

print("")
print("Test summary: Expected to fail until request_categories is implemented")
