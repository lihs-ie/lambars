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
    http_error_count = 0,       -- 4xx/5xx HTTP errors only
    network_error_count = 0,    -- Socket/connection errors (connect, read, write, timeout)
    timeout_count = 0,
    connect_error_count = 0,
    retry_count = 0,
    conflict_count = 0,

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
    M.state.http_error_count = 0
    M.state.network_error_count = 0
    M.state.timeout_count = 0
    M.state.connect_error_count = 0
    M.state.retry_count = 0
    M.state.conflict_count = 0
    M.state.errors_by_status = {}
    M.state.errors_by_type = {
        timeout = 0,
        connect = 0,
        read = 0,
        write = 0,
        other = 0,
    }
end

-- Track a response (thread-local, for best-effort tracking)
-- NOTE: Due to wrk's thread isolation, data collected here is NOT available in done().
-- For accurate counts, use aggregate_from_summary() in done() instead.
-- @param status number HTTP status code
-- @param error_type string|nil Error type (timeout, connect, read, write)
function M.track_response(status, error_type)
    M.state.total_requests = M.state.total_requests + 1

    -- Track HTTP errors (4xx, 5xx)
    if status >= 400 then
        M.state.http_error_count = M.state.http_error_count + 1

        -- Track by status code
        local status_key = tostring(status)
        M.state.errors_by_status[status_key] = (M.state.errors_by_status[status_key] or 0) + 1

        -- Track 409 Conflict responses separately
        if status == 409 then
            M.state.conflict_count = M.state.conflict_count + 1
        end
    end

    -- Track network/connection errors
    if error_type then
        M.state.network_error_count = M.state.network_error_count + 1
        if M.state.errors_by_type[error_type] then
            M.state.errors_by_type[error_type] = M.state.errors_by_type[error_type] + 1
        else
            M.state.errors_by_type.other = M.state.errors_by_type.other + 1
        end

        -- Track timeouts and connection errors specifically
        if error_type == "timeout" then
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

-- Get HTTP error rate (4xx/5xx only, excluding network errors)
-- @return number HTTP error rate (0.0-1.0)
function M.http_error_rate()
    if M.state.total_requests == 0 then
        return 0
    end
    return M.state.http_error_count / M.state.total_requests
end

-- Get network error rate (socket/connection errors only)
-- @return number Network error rate (0.0-1.0)
function M.network_error_rate()
    if M.state.total_requests == 0 then
        return 0
    end
    return M.state.network_error_count / M.state.total_requests
end

-- Get HTTP error rate (4xx/5xx only)
-- This is the primary error rate metric for threshold checking.
-- @return number HTTP error rate (0.0-1.0)
function M.error_rate()
    return M.http_error_rate()
end

-- Get total error rate (HTTP + network errors combined)
-- This is provided for backward compatibility and diagnostic purposes.
-- @return number Total error rate (0.0-1.0)
function M.total_error_rate()
    if M.state.total_requests == 0 then
        return 0
    end
    return (M.state.http_error_count + M.state.network_error_count) / M.state.total_requests
end

-- Get conflict rate (409 responses)
-- @return number Conflict rate (0.0-1.0)
function M.conflict_rate()
    if M.state.total_requests == 0 then
        return 0
    end
    return M.state.conflict_count / M.state.total_requests
end

-- Check if HTTP error rate is within threshold
-- Uses HTTP error rate (4xx/5xx) only, not network errors.
-- @return boolean True if within threshold or no threshold set
function M.is_within_threshold()
    if not M.state.expected_error_rate then
        return true -- No threshold set
    end
    return M.http_error_rate() <= M.state.expected_error_rate
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
-- NOTE: wrk's summary.errors.status counts non-2xx responses (including 3xx).
--       For accurate 4xx/5xx counts, use set_http_error_counts() with status distribution.
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

    -- Calculate total network/connection errors (socket errors only)
    M.state.network_error_count = (errors.connect or 0)
        + (errors.read or 0)
        + (errors.write or 0)
        + (errors.timeout or 0)

    -- Set total requests from summary
    M.state.total_requests = summary.requests or 0

    -- HTTP error count will be set by set_http_error_counts()
    M.state.http_error_count = 0

    -- Conflict count from response() is thread-local and will be 0 in done().
    -- wrk does not provide per-status-code counts in summary, so we cannot
    -- reliably count 409 responses. Reset to 0 with a note that this is a limitation.
    -- If single-thread mode is used, the thread-local value may be accurate.
    -- For multi-thread, consider using server-side logging for accurate conflict counts.
    M.state.conflict_count = 0
end

-- Set HTTP error counts from status distribution
-- Called from result_collector after categorizing status codes.
-- @param counts table Table with keys: total, count_400, count_404, count_409, count_422, count_500
function M.set_http_error_counts(counts)
    if not counts then
        return
    end

    M.state.http_error_count = counts.total or 0
    M.state.conflict_count = counts.count_409 or 0

    -- Update errors_by_status with individual status code counts
    if counts.count_400 and counts.count_400 > 0 then
        M.state.errors_by_status["400"] = counts.count_400
    end
    if counts.count_404 and counts.count_404 > 0 then
        M.state.errors_by_status["404"] = counts.count_404
    end
    if counts.count_409 and counts.count_409 > 0 then
        M.state.errors_by_status["409"] = counts.count_409
    end
    if counts.count_422 and counts.count_422 > 0 then
        M.state.errors_by_status["422"] = counts.count_422
    end
    if counts.count_500 and counts.count_500 > 0 then
        M.state.errors_by_status["500"] = counts.count_500
    end
end

-- Legacy function for backward compatibility
-- @param count number Number of HTTP errors (4xx, 5xx)
-- @deprecated Use set_http_error_counts() instead
function M.set_http_errors(count)
    M.state.http_error_count = count or 0
end

-- Get summary of error tracking
-- @return table Summary data
function M.get_summary()
    local current_http_error_rate = M.http_error_rate()
    local current_network_error_rate = M.network_error_rate()
    local current_total_error_rate = M.total_error_rate()
    local current_conflict_rate = M.conflict_rate()
    return {
        total_requests = M.state.total_requests,
        -- HTTP errors (4xx/5xx only)
        http_error_count = M.state.http_error_count,
        http_error_rate = current_http_error_rate,
        -- Network errors (socket/connection errors)
        network_error_count = M.state.network_error_count,
        network_error_rate = current_network_error_rate,
        -- Conflict metrics (409 responses)
        conflict_count = M.state.conflict_count,
        conflict_rate = current_conflict_rate,
        -- Primary error rate (HTTP errors only, used for threshold checking)
        error_rate = current_http_error_rate,
        -- Total error count and rate (HTTP + network errors, for diagnostics)
        error_count = M.state.http_error_count + M.state.network_error_count,
        total_error_rate = current_total_error_rate,
        -- Detailed breakdown
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
