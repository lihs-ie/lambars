-- test_result_collector_metrics.lua
-- REQ-MEASURE-401: result_collector.lua のメトリクス分離テスト

-- 純粋関数: メトリクス計算
local function calculate_metrics(status_distribution, excluded_requests)
    local tracked_requests = 0
    local success_count = 0  -- 2xx
    local conflict_count = 0  -- 409
    local error_count = 0  -- 4xx (excluding 409) + 5xx
    local server_error_count = 0  -- 5xx

    for status, count in pairs(status_distribution) do
        local status_num = tonumber(status)
        if status_num then
            tracked_requests = tracked_requests + count

            if status_num >= 200 and status_num < 300 then
                success_count = success_count + count
            elseif status_num == 409 then
                conflict_count = conflict_count + count
            elseif status_num >= 400 and status_num < 500 then
                error_count = error_count + count
            elseif status_num >= 500 then
                error_count = error_count + count
                server_error_count = server_error_count + count
            end
        end
    end

    local metrics = {}
    if tracked_requests > 0 then
        metrics.success_rate = success_count / tracked_requests
        metrics.conflict_rate = conflict_count / tracked_requests
        metrics.error_rate = error_count / tracked_requests
        metrics.server_error_rate = server_error_count / tracked_requests
    else
        metrics.success_rate = 0
        metrics.conflict_rate = 0
        metrics.error_rate = 0
        metrics.server_error_rate = 0
    end

    metrics.tracked_requests = tracked_requests
    metrics.excluded_requests = excluded_requests
    metrics.total_requests = tracked_requests + excluded_requests

    return metrics
end

local function test_calculate_metrics_normal_case()
    -- 通常ケース
    local status_distribution = {
        ["200"] = 80,
        ["201"] = 10,
        ["409"] = 5,
        ["400"] = 3,
        ["500"] = 2
    }
    local excluded_requests = 20  -- backoff + suppressed + fallback

    local metrics = calculate_metrics(status_distribution, excluded_requests)

    assert(metrics.tracked_requests == 100, string.format("tracked_requests should be 100, got %d", metrics.tracked_requests))
    assert(metrics.excluded_requests == 20, string.format("excluded_requests should be 20, got %d", metrics.excluded_requests))
    assert(metrics.total_requests == 120, string.format("total_requests should be 120, got %d", metrics.total_requests))

    -- success_rate = (80 + 10) / 100 = 0.90
    assert(math.abs(metrics.success_rate - 0.90) < 0.01, string.format("success_rate should be ~0.90, got %.2f", metrics.success_rate))

    -- conflict_rate = 5 / 100 = 0.05
    assert(math.abs(metrics.conflict_rate - 0.05) < 0.01, string.format("conflict_rate should be ~0.05, got %.2f", metrics.conflict_rate))

    -- error_rate = (3 + 2) / 100 = 0.05
    assert(math.abs(metrics.error_rate - 0.05) < 0.01, string.format("error_rate should be ~0.05, got %.2f", metrics.error_rate))

    -- server_error_rate = 2 / 100 = 0.02
    assert(math.abs(metrics.server_error_rate - 0.02) < 0.01, string.format("server_error_rate should be ~0.02, got %.2f", metrics.server_error_rate))

    print("PASS: calculate_metrics normal case test")
end

local function test_calculate_metrics_zero_tracked()
    -- tracked_requests が 0 のケース
    local status_distribution = {}
    local excluded_requests = 50

    local metrics = calculate_metrics(status_distribution, excluded_requests)

    assert(metrics.tracked_requests == 0, "tracked_requests should be 0")
    assert(metrics.excluded_requests == 50, "excluded_requests should be 50")
    assert(metrics.total_requests == 50, "total_requests should be 50")
    assert(metrics.success_rate == 0, "success_rate should be 0")
    assert(metrics.conflict_rate == 0, "conflict_rate should be 0")
    assert(metrics.error_rate == 0, "error_rate should be 0")
    assert(metrics.server_error_rate == 0, "server_error_rate should be 0")

    print("PASS: calculate_metrics zero tracked test")
end

local function test_calculate_metrics_high_conflict()
    -- 高conflict率のケース
    local status_distribution = {
        ["200"] = 50,
        ["409"] = 40,  -- 高conflict率
        ["500"] = 10
    }
    local excluded_requests = 0

    local metrics = calculate_metrics(status_distribution, excluded_requests)

    assert(metrics.tracked_requests == 100, "tracked_requests should be 100")
    assert(math.abs(metrics.success_rate - 0.50) < 0.01, "success_rate should be ~0.50")
    assert(math.abs(metrics.conflict_rate - 0.40) < 0.01, "conflict_rate should be ~0.40")
    assert(math.abs(metrics.error_rate - 0.10) < 0.01, "error_rate should be ~0.10")
    assert(math.abs(metrics.server_error_rate - 0.10) < 0.01, "server_error_rate should be ~0.10")

    print("PASS: calculate_metrics high conflict test")
end

-- テスト実行
print("Running result_collector metrics tests...")
print("")

test_calculate_metrics_normal_case()
test_calculate_metrics_zero_tracked()
test_calculate_metrics_high_conflict()

print("")
print("Test summary: All tests passed")
