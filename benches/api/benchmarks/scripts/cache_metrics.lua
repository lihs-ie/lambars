-- Cache metrics tracking module
-- benches/api/benchmarks/scripts/cache_metrics.lua
--
-- Tracks cache hit/miss rates and latency distribution
-- for benchmark scenarios.
--
-- Usage:
--   local cache_metrics = require("cache_metrics")
--   cache_metrics.init()
--
--   -- In response():
--   cache_metrics.track(endpoint, is_cache_hit, latency_us)
--
--   -- In done():
--   local summary = cache_metrics.get_summary()

local M = {}

-- State
M.state = {
    enabled = false,
    per_endpoint = false,
    track_latency = false,
    warmup_requests = 0,
    expected_hit_rate = nil,

    -- Metrics
    total_requests = 0,
    cache_hits = 0,
    cache_misses = 0,
    warmup_done = false,

    -- Per-endpoint metrics
    endpoints = {},

    -- Latency tracking
    hit_latencies = {},
    miss_latencies = {},
}

-- Initialize from environment variables
function M.init()
    M.state.enabled = os.getenv("CACHE_METRICS_ENABLED") == "1"
    M.state.per_endpoint = os.getenv("CACHE_METRICS_PER_ENDPOINT") == "1"
    M.state.track_latency = os.getenv("CACHE_METRICS_TRACK_LATENCY") == "1"
    M.state.warmup_requests = tonumber(os.getenv("CACHE_WARMUP_REQUESTS") or "0") or 0

    local expected = os.getenv("EXPECTED_CACHE_HIT_RATE")
    if expected then
        M.state.expected_hit_rate = tonumber(expected)
    end

    -- Reset state
    M.state.total_requests = 0
    M.state.cache_hits = 0
    M.state.cache_misses = 0
    M.state.warmup_done = false
    M.state.endpoints = {}
    M.state.hit_latencies = {}
    M.state.miss_latencies = {}
end

-- Track a response with cache info
-- @param endpoint string The endpoint path
-- @param is_cache_hit boolean Whether the response was from cache
-- @param latency_us number Response latency in microseconds
function M.track(endpoint, is_cache_hit, latency_us)
    if not M.state.enabled then return end

    M.state.total_requests = M.state.total_requests + 1

    -- Skip during warmup
    if M.state.total_requests <= M.state.warmup_requests then
        return
    end

    if not M.state.warmup_done then
        M.state.warmup_done = true
        -- Reset counters after warmup
        M.state.cache_hits = 0
        M.state.cache_misses = 0
    end

    if is_cache_hit then
        M.state.cache_hits = M.state.cache_hits + 1
        if M.state.track_latency then
            table.insert(M.state.hit_latencies, latency_us)
        end
    else
        M.state.cache_misses = M.state.cache_misses + 1
        if M.state.track_latency then
            table.insert(M.state.miss_latencies, latency_us)
        end
    end

    -- Per-endpoint tracking
    if M.state.per_endpoint then
        if not M.state.endpoints[endpoint] then
            M.state.endpoints[endpoint] = { hits = 0, misses = 0 }
        end
        if is_cache_hit then
            M.state.endpoints[endpoint].hits = M.state.endpoints[endpoint].hits + 1
        else
            M.state.endpoints[endpoint].misses = M.state.endpoints[endpoint].misses + 1
        end
    end
end

-- Get overall cache hit rate
function M.hit_rate()
    local total = M.state.cache_hits + M.state.cache_misses
    if total == 0 then return 0 end
    return M.state.cache_hits / total
end

-- Get metrics summary as table
function M.get_summary()
    local total = M.state.cache_hits + M.state.cache_misses
    local hit_rate = M.hit_rate()

    -- Determine if warmup is completed
    -- Warmup is complete if:
    -- 1. warmup_requests is 0 (no warmup needed), or
    -- 2. warmup_done flag is set (warmup phase passed)
    local warmup_completed = M.state.warmup_requests == 0 or M.state.warmup_done

    -- Determine if hit rate threshold is met
    -- hit_rate_met is:
    -- - nil if no samples after warmup (cannot evaluate)
    -- - nil if expected_hit_rate is nil (no threshold set)
    -- - true/false based on comparison otherwise
    local hit_rate_met = nil
    if total > 0 and M.state.expected_hit_rate ~= nil then
        hit_rate_met = hit_rate >= M.state.expected_hit_rate
    end

    local summary = {
        enabled = M.state.enabled,
        warmup_completed = warmup_completed,
        warmup_requests = M.state.warmup_requests,
        total_requests = total,
        cache_hits = M.state.cache_hits,
        cache_misses = M.state.cache_misses,
        hit_rate = hit_rate,
        expected_hit_rate = M.state.expected_hit_rate,
        hit_rate_met = hit_rate_met,
    }

    if M.state.per_endpoint then
        summary.per_endpoint = {}
        for endpoint, stats in pairs(M.state.endpoints) do
            local ep_total = stats.hits + stats.misses
            summary.per_endpoint[endpoint] = {
                hits = stats.hits,
                misses = stats.misses,
                hit_rate = ep_total > 0 and (stats.hits / ep_total) or 0,
            }
        end
    end

    if M.state.track_latency then
        summary.latency = {
            hit_avg_us = M.average(M.state.hit_latencies),
            miss_avg_us = M.average(M.state.miss_latencies),
            hit_p99_us = M.percentile(M.state.hit_latencies, 99),
            miss_p99_us = M.percentile(M.state.miss_latencies, 99),
        }
    end

    return summary
end

-- Helper: calculate average
function M.average(arr)
    if #arr == 0 then return 0 end
    local sum = 0
    for _, v in ipairs(arr) do sum = sum + v end
    return sum / #arr
end

-- Helper: calculate percentile
function M.percentile(arr, p)
    if #arr == 0 then return 0 end
    -- Create a copy to avoid modifying the original array
    local sorted = {}
    for i, v in ipairs(arr) do sorted[i] = v end
    table.sort(sorted)
    local idx = math.ceil(#sorted * p / 100)
    return sorted[idx] or sorted[#sorted]
end

-- Reset all metrics
function M.reset()
    M.state.total_requests = 0
    M.state.cache_hits = 0
    M.state.cache_misses = 0
    M.state.warmup_done = false
    M.state.endpoints = {}
    M.state.hit_latencies = {}
    M.state.miss_latencies = {}
end

return M
