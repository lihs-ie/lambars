-- Error tracking module for wrk benchmark scripts

local M = {}

M.threads = {}
M.state = {
    timeout_ms = 30000,
    connect_timeout_ms = 5000,
    max_retries = 0,
    retry_delay_ms = 1000,
    expected_error_rate = nil,
    fail_on_error_threshold = false,
    inject_error_rate = nil,
    total_requests = 0,
    http_error_count = 0,
    network_error_count = 0,
    timeout_count = 0,
    connect_error_count = 0,
    retry_count = 0,
    conflict_count = 0,
    errors_by_status = {},
    errors_by_type = {timeout = 0, connect = 0, read = 0, write = 0, other = 0},
}

function M.init()
    M.state.timeout_ms = tonumber(os.getenv("REQUEST_TIMEOUT_MS")) or 30000
    M.state.connect_timeout_ms = tonumber(os.getenv("CONNECT_TIMEOUT_MS")) or 5000
    M.state.max_retries = tonumber(os.getenv("MAX_RETRIES")) or 0
    M.state.retry_delay_ms = tonumber(os.getenv("RETRY_DELAY_MS")) or 1000
    M.state.expected_error_rate = tonumber(os.getenv("EXPECTED_ERROR_RATE"))
    M.state.fail_on_error_threshold = os.getenv("FAIL_ON_ERROR_THRESHOLD") == "1"
    M.state.inject_error_rate = tonumber(os.getenv("INJECT_ERROR_RATE"))
    M.state.total_requests = 0
    M.state.http_error_count = 0
    M.state.network_error_count = 0
    M.state.timeout_count = 0
    M.state.connect_error_count = 0
    M.state.retry_count = 0
    M.state.conflict_count = 0
    M.state.errors_by_status = {}
    M.state.errors_by_type = {timeout = 0, connect = 0, read = 0, write = 0, other = 0}
end

local function safe_rate(count)
    return M.state.total_requests == 0 and 0 or count / M.state.total_requests
end

function M.track_response(status, error_type)
    M.state.total_requests = M.state.total_requests + 1

    if status >= 400 then
        M.state.http_error_count = M.state.http_error_count + 1
        local status_key = tostring(status)
        M.state.errors_by_status[status_key] = (M.state.errors_by_status[status_key] or 0) + 1
        if status == 409 then M.state.conflict_count = M.state.conflict_count + 1 end
    end

    if error_type then
        M.state.network_error_count = M.state.network_error_count + 1
        M.state.errors_by_type[error_type] = (M.state.errors_by_type[error_type] or 0) + 1
        if error_type == "timeout" then
            M.state.timeout_count = M.state.timeout_count + 1
        elseif error_type == "connect" then
            M.state.connect_error_count = M.state.connect_error_count + 1
        end
    end
end

function M.track_retry() M.state.retry_count = M.state.retry_count + 1 end
function M.should_inject_error() return M.state.inject_error_rate and math.random() < M.state.inject_error_rate end
function M.http_error_rate() return safe_rate(M.state.http_error_count) end
function M.network_error_rate() return safe_rate(M.state.network_error_count) end
function M.error_rate() return M.http_error_rate() end
function M.total_error_rate() return safe_rate(M.state.http_error_count + M.state.network_error_count) end
function M.conflict_rate() return safe_rate(M.state.conflict_count) end
function M.is_within_threshold() return not M.state.expected_error_rate or M.http_error_rate() <= M.state.expected_error_rate end
function M.should_fail() return M.state.fail_on_error_threshold and not M.is_within_threshold() end

function M.aggregate_from_summary(summary)
    if not summary then return end

    local errors = summary.errors or {}
    M.state.connect_error_count = errors.connect or 0
    M.state.timeout_count = errors.timeout or 0
    M.state.errors_by_type = {
        connect = errors.connect or 0,
        read = errors.read or 0,
        write = errors.write or 0,
        timeout = errors.timeout or 0,
        other = 0,
    }
    M.state.network_error_count = (errors.connect or 0) + (errors.read or 0) +
                                   (errors.write or 0) + (errors.timeout or 0)
    M.state.total_requests = summary.requests or 0
    M.state.http_error_count = 0
    M.state.conflict_count = 0
end

function M.set_http_error_counts(counts)
    if not counts then return end
    M.state.http_error_count = counts.total or 0
    M.state.conflict_count = counts.count_409 or 0
    for _, code in ipairs({"400", "404", "409", "422", "500"}) do
        local count = counts["count_" .. code]
        if count and count > 0 then M.state.errors_by_status[code] = count end
    end
end

function M.set_http_errors(count) M.state.http_error_count = count or 0 end

function M.get_summary()
    return {
        total_requests = M.state.total_requests,
        http_error_count = M.state.http_error_count,
        http_error_rate = M.http_error_rate(),
        network_error_count = M.state.network_error_count,
        network_error_rate = M.network_error_rate(),
        conflict_count = M.state.conflict_count,
        conflict_rate = M.conflict_rate(),
        error_rate = M.http_error_rate(),
        error_count = M.state.http_error_count + M.state.network_error_count,
        total_error_rate = M.total_error_rate(),
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

function M.setup_thread(thread)
    for _, code in ipairs({"200", "201", "207", "400", "404", "409", "422", "500", "502", "other"}) do
        thread:set("status_" .. code, 0)
    end
end

function M.track_thread_response(status)
    local thread = wrk.thread
    if not thread then return end

    local key = "status_" .. tostring(status)
    thread:set(key, (tonumber(thread:get(key)) or 0) + 1)

    local is_standard = false
    for _, code in ipairs({200, 201, 207, 400, 404, 409, 422, 500, 502}) do
        if status == code then is_standard = true break end
    end
    if not is_standard and status >= 400 then
        thread:set("status_other", (tonumber(thread:get("status_other")) or 0) + 1)
    end
end

function M.get_thread_aggregated_summary()
    local thread = wrk.thread
    if not thread then
        return {
            status_200 = 0, status_201 = 0, status_207 = 0,
            status_400 = 0, status_404 = 0, status_409 = 0,
            status_422 = 0, status_500 = 0, status_502 = 0,
            status_other = 0,
        }
    end

    local aggregated = {}
    for _, code in ipairs({"200", "201", "207", "400", "404", "409", "422", "500", "502", "other"}) do
        aggregated["status_" .. code] = tonumber(thread:get("status_" .. code)) or 0
    end
    return aggregated
end

return M
