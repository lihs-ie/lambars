-- Error tracking module for benchmark scenarios
-- benches/api/benchmarks/scripts/error_tracker.lua
--
-- Tracks error rates, timeouts, and provides chaos testing support.
--
-- Usage:
--   local error_tracker = require("error_tracker")
--   error_tracker.init()
--
--   -- In response():
--   error_tracker.track_response(status, error_type)
--
--   -- In done():
--   local summary = error_tracker.get_summary()

local M = {}

-- State
M.state = {
    -- Configuration (loaded from environment)
    timeout_ms = 30000,
    connect_timeout_ms = 5000,
    max_retries = 0,
    retry_delay_ms = 1000,
    expected_error_rate = nil,
    fail_on_error_threshold = false,
    inject_error_rate = nil,

    -- Metrics
    total_requests = 0,
    error_count = 0,
    timeout_count = 0,
    connect_error_count = 0,
    retry_count = 0,

    -- Error categories
    errors_by_status = {},
    errors_by_type = {
        timeout = 0,
        connect = 0,
        read = 0,
        write = 0,
        other = 0,
    },
}

-- Initialize from environment variables
function M.init()
    M.state.timeout_ms = tonumber(os.getenv("REQUEST_TIMEOUT_MS") or "30000") or 30000
    M.state.connect_timeout_ms = tonumber(os.getenv("CONNECT_TIMEOUT_MS") or "5000") or 5000
    M.state.max_retries = tonumber(os.getenv("MAX_RETRIES") or "0") or 0
    M.state.retry_delay_ms = tonumber(os.getenv("RETRY_DELAY_MS") or "1000") or 1000

    local expected = os.getenv("EXPECTED_ERROR_RATE")
    if expected then
        M.state.expected_error_rate = tonumber(expected)
    end

    M.state.fail_on_error_threshold = os.getenv("FAIL_ON_ERROR_THRESHOLD") == "1"

    local inject = os.getenv("INJECT_ERROR_RATE")
    if inject then
        M.state.inject_error_rate = tonumber(inject)
    end

    -- Reset counters
    M.state.total_requests = 0
    M.state.error_count = 0
    M.state.timeout_count = 0
    M.state.connect_error_count = 0
    M.state.retry_count = 0
    M.state.errors_by_status = {}
    M.state.errors_by_type = {
        timeout = 0,
        connect = 0,
        read = 0,
        write = 0,
        other = 0,
    }
end

-- Track a response
-- @param status number HTTP status code
-- @param error_type string|nil Error type (timeout, connect, read, write)
function M.track_response(status, error_type)
    M.state.total_requests = M.state.total_requests + 1

    if status >= 400 or error_type then
        M.state.error_count = M.state.error_count + 1

        -- Track by status code
        local status_key = tostring(status)
        M.state.errors_by_status[status_key] = (M.state.errors_by_status[status_key] or 0) + 1

        -- Track by error type
        if error_type then
            if M.state.errors_by_type[error_type] then
                M.state.errors_by_type[error_type] = M.state.errors_by_type[error_type] + 1
            else
                M.state.errors_by_type.other = M.state.errors_by_type.other + 1
            end
        end

        -- Track timeouts specifically
        if status == 408 or error_type == "timeout" then
            M.state.timeout_count = M.state.timeout_count + 1
        elseif error_type == "connect" then
            M.state.connect_error_count = M.state.connect_error_count + 1
        end
    end
end

-- Track a retry attempt
function M.track_retry()
    M.state.retry_count = M.state.retry_count + 1
end

-- Check if we should inject an error (for chaos testing)
-- @return boolean True if an error should be injected
function M.should_inject_error()
    if not M.state.inject_error_rate then
        return false
    end
    return math.random() < M.state.inject_error_rate
end

-- Get error rate
-- @return number Error rate (0.0-1.0)
function M.error_rate()
    if M.state.total_requests == 0 then
        return 0
    end
    return M.state.error_count / M.state.total_requests
end

-- Check if error rate is within threshold
-- @return boolean True if within threshold or no threshold set
function M.is_within_threshold()
    if not M.state.expected_error_rate then
        return true -- No threshold set
    end
    return M.error_rate() <= M.state.expected_error_rate
end

-- Check if test should fail due to error rate
-- @return boolean True if test should fail
function M.should_fail()
    if not M.state.fail_on_error_threshold then
        return false
    end
    return not M.is_within_threshold()
end

-- Aggregate error counts from wrk's summary object
-- This function should be called from done() to get accurate, thread-aggregated error counts.
-- @param summary table wrk's summary object containing errors.connect, errors.read, etc.
function M.aggregate_from_summary(summary)
    if not summary then
        return
    end

    local errors = summary.errors or {}

    -- Reset and populate from wrk summary (thread-aggregated, accurate)
    M.state.connect_error_count = errors.connect or 0
    M.state.timeout_count = errors.timeout or 0

    M.state.errors_by_type = {
        connect = errors.connect or 0,
        read = errors.read or 0,
        write = errors.write or 0,
        timeout = errors.timeout or 0,
        other = 0,  -- wrk does not provide "other" category
    }

    -- Calculate total network/connection errors
    local total_network_errors = (errors.connect or 0)
        + (errors.read or 0)
        + (errors.write or 0)
        + (errors.timeout or 0)

    -- Set total requests from summary
    M.state.total_requests = summary.requests or 0

    -- Initialize error_count with network errors
    -- HTTP errors (4xx, 5xx) will be added by set_http_errors()
    M.state.error_count = total_network_errors
end

-- Add HTTP error count to the total
-- Called from result_collector after calculating HTTP errors from status_distribution
-- @param count number Number of HTTP errors (4xx, 5xx)
function M.set_http_errors(count)
    M.state.error_count = M.state.error_count + (count or 0)
end

-- Get summary of error tracking
-- @return table Summary data
function M.get_summary()
    local current_error_rate = M.error_rate()
    return {
        total_requests = M.state.total_requests,
        error_count = M.state.error_count,
        error_rate = current_error_rate,
        timeout_count = M.state.timeout_count,
        connect_error_count = M.state.connect_error_count,
        retry_count = M.state.retry_count,
        errors_by_status = M.state.errors_by_status,
        errors_by_type = M.state.errors_by_type,
        expected_error_rate = M.state.expected_error_rate,
        within_threshold = M.is_within_threshold(),
        fail_on_threshold = M.state.fail_on_error_threshold,
        should_fail = M.should_fail(),
        config = {
            timeout_ms = M.state.timeout_ms,
            connect_timeout_ms = M.state.connect_timeout_ms,
            max_retries = M.state.max_retries,
            retry_delay_ms = M.state.retry_delay_ms,
            inject_error_rate = M.state.inject_error_rate,
        },
    }
end

-- Get configuration
-- @return table Configuration data
function M.get_config()
    return {
        timeout_ms = M.state.timeout_ms,
        connect_timeout_ms = M.state.connect_timeout_ms,
        max_retries = M.state.max_retries,
        retry_delay_ms = M.state.retry_delay_ms,
        expected_error_rate = M.state.expected_error_rate,
        fail_on_error_threshold = M.state.fail_on_error_threshold,
        inject_error_rate = M.state.inject_error_rate,
    }
end

return M
