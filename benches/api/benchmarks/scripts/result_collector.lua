local M = {}

M.NULL = {}

local cache_metrics_ok, cache_metrics = pcall(require, "cache_metrics")
local error_tracker_ok, error_tracker = pcall(require, "error_tracker")
if not cache_metrics_ok then cache_metrics = nil end
if not error_tracker_ok then error_tracker = nil end

M.config = {
    scenario_name = "benchmark",
    output_format = "json",
    output_file = nil,
    include_raw_latencies = false
}

M.results = {
    scenario = { name = nil, storage_mode = nil, cache_mode = nil, load_pattern = nil, contention_level = nil },
    execution = { timestamp = nil, duration_seconds = 0, threads = 0, connections = 0 },
    timestamp = nil,
    duration_seconds = 0,
    total_requests = 0,
    successful_requests = 0,
    failed_requests = 0,
    error_rate = 0,
    http_error_rate = 0,
    network_error_rate = 0,
    server_error_rate = 0,
    client_error_rate = 0,
    conflict_rate = 0,
    conflict_count = 0,
    status_code_counts = { count_400 = 0, count_404 = 0, count_409 = 0, count_422 = 0, count_500 = 0 },
    http_status = {},
    retries = 0,
    meta = {
        tracked_requests = 0,
        excluded_requests = 0,
        success_rate = 0
    },
    rps = { target = 0, actual = 0 },
    latency = {
        min_us = 0, max_us = 0, mean_us = 0, stdev_us = 0,
        min_ms = 0, max_ms = 0, mean_ms = 0, stddev_ms = 0,
        percentiles = { p50 = M.NULL, p75 = M.NULL, p90 = M.NULL, p95 = M.NULL, p99 = M.NULL, p99_9 = M.NULL },
        p50_ms = M.NULL, p75_ms = M.NULL, p90_ms = M.NULL, p95_ms = M.NULL, p99_ms = M.NULL, p999_ms = M.NULL
    },
    throughput = { requests_total = 0, requests_per_second = 0, bytes_total = 0, bytes_per_second = 0 },
    payload = { variant = nil, estimated_size_bytes = 0 },
    load_profile = { profile = nil, target_rps = 0 },
    status_distribution = {},
    status_codes = {},
    errors = { connect = 0, read = 0, write = 0, timeout = 0, status = { ["4xx"] = 0, ["5xx"] = 0 } },
    cache = nil,
    errors_detail = nil
}

M.response_count = 0
M.error_count = 0
M.status_counts = {}
M.retry_count = 0
M.start_time = nil
M.current_endpoint = nil

function M.init(options)
    options = options or {}
    for key, value in pairs(options) do
        if M.config[key] ~= nil then M.config[key] = value end
    end

    local timestamp = os.date("!%Y-%m-%dT%H:%M:%SZ")
    M.results.scenario.name = M.config.scenario_name
    M.results.execution.timestamp = timestamp
    M.results.timestamp = timestamp
    M.start_time = os.time()

    M.results.execution.threads = tonumber(os.getenv("THREADS")) or 0
    M.results.execution.connections = tonumber(os.getenv("CONNECTIONS")) or 0

    M.load_scenario_from_env()

    local env_scenario_name = os.getenv("SCENARIO_NAME")
    if env_scenario_name and env_scenario_name ~= "" and M.results.scenario.name == "benchmark" then
        M.results.scenario.name = env_scenario_name
    end

    M.response_count = 0
    M.error_count = 0
    M.status_counts = {}
    M.retry_count = 0
    M.results.http_status = {}
    M.results.retries = 0

    if cache_metrics then cache_metrics.init() end
    if error_tracker then error_tracker.init() end
end

function M.set_scenario_metadata(metadata)
    if not metadata then return end
    M.results.scenario.name = metadata.name or M.results.scenario.name
    M.results.scenario.storage_mode = metadata.storage_mode
    M.results.scenario.cache_mode = metadata.cache_mode
    M.results.scenario.load_pattern = metadata.load_pattern
    M.results.scenario.contention_level = metadata.contention_level
end

function M.set_request_categories(categories)
    if not categories then return end
    local excluded = (categories.backoff or 0) + (categories.suppressed or 0) + (categories.fallback or 0)
    M.results.meta.excluded_requests = excluded
end

function M.load_scenario_from_env()
    M.results.scenario.storage_mode = os.getenv("STORAGE_MODE")
    M.results.scenario.cache_mode = os.getenv("CACHE_MODE")
    M.results.scenario.load_pattern = os.getenv("LOAD_PATTERN")
    M.results.scenario.contention_level = os.getenv("CONTENTION_LEVEL")
end

function M.record_response(status, latency_us, headers, endpoint)
    M.response_count = M.response_count + 1
    local status_key = tostring(status)
    M.status_counts[status_key] = (M.status_counts[status_key] or 0) + 1
    if status >= 400 then M.error_count = M.error_count + 1 end

    if error_tracker then error_tracker.track_response(status, nil) end

    if cache_metrics and cache_metrics.state.enabled then
        local effective_endpoint = endpoint or M.current_endpoint or "unknown"
        cache_metrics.track(effective_endpoint, M.detect_cache_hit(headers), latency_us or 0)
    end
end

function M.track_retry()
    M.retry_count = M.retry_count + 1
    if error_tracker then error_tracker.track_retry() end
end

function M.set_current_endpoint(endpoint)
    M.current_endpoint = endpoint
end

function M.detect_cache_hit(headers)
    if not headers then return false end

    local function check_header(name, pattern)
        local value = headers[name] or headers[string.lower(name)]
        return value and string.find(string.lower(value), pattern)
    end

    if check_header("X-Cache", "hit") then return true end
    if check_header("X-Cache-Status", "hit") then return true end
    if check_header("CF-Cache-Status", "hit") then return true end

    local x_cache_hit = headers["X-Cache-Hit"] or headers["x-cache-hit"]
    if x_cache_hit and (x_cache_hit == "true" or x_cache_hit == "1") then return true end

    local age = headers["Age"] or headers["age"]
    if age then
        local age_value = tonumber(age)
        if age_value and age_value > 0 then return true end
    end

    return false
end

function M.set_load_profile(profile_metadata)
    if not profile_metadata then return end
    M.results.load_profile = profile_metadata
    M.results.rps.target = profile_metadata.target_rps or 0
end

function M.set_payload(payload_metadata)
    if payload_metadata then M.results.payload = payload_metadata end
end

local function calculate_thread_requests()
    local total = 0
    for _, count in pairs(M.status_counts) do
        if type(count) == "number" then total = total + count end
    end
    return total
end

local function calculate_failed_requests()
    local failed = 0
    for code, count in pairs(M.status_counts) do
        local code_num = tonumber(code)
        if code_num and code_num >= 400 and type(count) == "number" then
            failed = failed + count
        end
    end
    return failed
end

function M.finalize(summary, latency, requests)
    if not summary then
        M.results.total_requests = M.response_count
        M.results.failed_requests = M.error_count
        M.results.successful_requests = M.response_count - M.error_count
        M.results.duration_seconds = os.time() - (M.start_time or os.time())
        M.results.execution.duration_seconds = M.results.duration_seconds
        return
    end

    local duration_seconds = summary.duration / 1000000
    M.results.duration_seconds = duration_seconds
    M.results.execution.duration_seconds = duration_seconds

    local thread_total_requests = calculate_thread_requests()
    local use_thread_local = thread_total_requests > 0
    M.results.total_requests = use_thread_local and thread_total_requests or summary.requests

    if use_thread_local then
        local thread_failed_requests = calculate_failed_requests()
        M.results.failed_requests = thread_failed_requests
        M.results.successful_requests = thread_total_requests - thread_failed_requests
    else
        M.results.successful_requests = summary.requests - (summary.errors.connect + summary.errors.read +
                                                             summary.errors.write + summary.errors.timeout +
                                                             summary.errors.status)
        M.results.failed_requests = summary.requests - M.results.successful_requests
    end

    if duration_seconds > 0 then
        M.results.rps.actual = M.results.total_requests / duration_seconds
    end

    M.results.throughput.requests_total = M.results.total_requests
    M.results.throughput.requests_per_second = M.results.rps.actual

    if use_thread_local then
        M.results.throughput.bytes_total = M.NULL
        M.results.throughput.bytes_per_second = M.NULL
        M.results.errors = {
            connect = M.NULL, read = M.NULL, write = M.NULL, timeout = M.NULL,
            status = { ["4xx"] = 0, ["5xx"] = 0 }
        }
        M.results.errors.status_total = M.NULL
    else
        M.results.throughput.bytes_total = summary.bytes or 0
        if duration_seconds > 0 then
            M.results.throughput.bytes_per_second = (summary.bytes or 0) / duration_seconds
        end
        M.results.errors = {
            connect = summary.errors.connect, read = summary.errors.read,
            write = summary.errors.write, timeout = summary.errors.timeout,
            status = { ["4xx"] = 0, ["5xx"] = 0 }
        }
        M.results.errors.status_total = summary.errors.status
    end

    if next(M.status_counts) then
        M.results.status_codes = M.status_counts
        M.results.status_distribution = M.status_counts

        for code, count in pairs(M.status_counts) do
            local code_num = tonumber(code)
            if code_num then
                if code_num >= 400 and code_num < 500 then
                    M.results.errors.status["4xx"] = (M.results.errors.status["4xx"] or 0) + count
                elseif code_num >= 500 then
                    M.results.errors.status["5xx"] = (M.results.errors.status["5xx"] or 0) + count
                end
            end
        end
    else
        M.results.status_codes = {
            ["2xx"] = M.results.successful_requests,
            ["errors"] = M.results.failed_requests,
            ["_note"] = "Detailed status code distribution unavailable due to wrk thread isolation"
        }
        M.results.status_distribution = "unknown"
    end

    if latency then
        M.results.latency.min_us = latency.min
        M.results.latency.max_us = latency.max
        M.results.latency.mean_us = latency.mean
        M.results.latency.stdev_us = latency.stdev
        M.results.latency.min_ms = latency.min / 1000
        M.results.latency.max_ms = latency.max / 1000
        M.results.latency.mean_ms = latency.mean / 1000
        M.results.latency.stddev_ms = latency.stdev / 1000

        local p50 = latency:percentile(50)
        local p75 = latency:percentile(75)
        local p90 = latency:percentile(90)
        local p95 = latency:percentile(95)
        local p99 = latency:percentile(99)
        local p999 = latency:percentile(99.9)

        M.results.latency.percentiles = {
            p50 = p50 or M.NULL, p75 = p75 or M.NULL, p90 = p90 or M.NULL,
            p95 = p95 or M.NULL, p99 = p99 or M.NULL, p99_9 = p999 or M.NULL
        }

        M.results.latency.p50_ms = p50 and (p50 / 1000) or M.NULL
        M.results.latency.p75_ms = p75 and (p75 / 1000) or M.NULL
        M.results.latency.p90_ms = p90 and (p90 / 1000) or M.NULL
        M.results.latency.p95_ms = p95 and (p95 / 1000) or M.NULL
        M.results.latency.p99_ms = p99 and (p99 / 1000) or M.NULL
        M.results.latency.p999_ms = p999 and (p999 / 1000) or M.NULL
    else
        M.results.latency.min_us = 0
        M.results.latency.max_us = 0
        M.results.latency.mean_us = 0
        M.results.latency.stdev_us = 0
        M.results.latency.min_ms = 0
        M.results.latency.max_ms = 0
        M.results.latency.mean_ms = 0
        M.results.latency.stddev_ms = 0
        M.results.latency.percentiles = {
            p50 = M.NULL, p75 = M.NULL, p90 = M.NULL, p95 = M.NULL, p99 = M.NULL, p99_9 = M.NULL
        }
        M.results.latency.p50_ms = M.NULL
        M.results.latency.p75_ms = M.NULL
        M.results.latency.p90_ms = M.NULL
        M.results.latency.p95_ms = M.NULL
        M.results.latency.p99_ms = M.NULL
        M.results.latency.p999_ms = M.NULL
    end

    if cache_metrics then M.results.cache = cache_metrics.get_summary() end

local function aggregate_error_counts()
    local counts = {
        total = 0, count_400 = 0, count_404 = 0, count_409 = 0, count_422 = 0, count_500 = 0,
        count_4xx_total = 0, count_4xx_excluding_409 = 0, count_5xx_total = 0
    }

    if not M.results.status_distribution or type(M.results.status_distribution) ~= "table" or not next(M.results.status_distribution) then
        return counts
    end

    local status_map = {[400] = "count_400", [404] = "count_404", [409] = "count_409", [422] = "count_422", [500] = "count_500"}

    for status, count in pairs(M.results.status_distribution) do
        local status_num = tonumber(status)
        local is_number = status_num and type(count) == "number"

        if is_number then
            if status_num >= 400 and status_num < 500 then
                counts.count_4xx_total = counts.count_4xx_total + count
                counts.total = counts.total + count
                if status_num ~= 409 then
                    counts.count_4xx_excluding_409 = counts.count_4xx_excluding_409 + count
                end
            elseif status_num >= 500 and status_num < 600 then
                counts.count_5xx_total = counts.count_5xx_total + count
                counts.total = counts.total + count
            end

            local key = status_map[status_num]
            if key then counts[key] = count end
        elseif status == "other" and type(count) == "number" then
            counts.count_5xx_total = counts.count_5xx_total + count
            counts.total = counts.total + count
        end
    end
    return counts
end

local function calculate_tracked_requests(status_distribution)
    local tracked = 0
    if status_distribution and type(status_distribution) == "table" then
        for status, count in pairs(status_distribution) do
            if (tonumber(status) or status == "other") and type(count) == "number" then
                tracked = tracked + count
            end
        end
    end
    return tracked
end

local function set_error_metrics(http_error_counts, summary)
    local tracked_requests = calculate_tracked_requests(M.results.status_distribution)
    if tracked_requests <= 0 and summary and summary.requests then
        local network_errors = (summary.errors and summary.errors.connect or 0) +
                               (summary.errors and summary.errors.read or 0) +
                               (summary.errors and summary.errors.write or 0) +
                               (summary.errors and summary.errors.timeout or 0)
        local excluded = M.results.meta.excluded_requests or 0
        local executed = math.max(0, summary.requests - network_errors - excluded)
        tracked_requests = math.max(executed, tracked_requests)
    end
    M.results.meta.tracked_requests = tracked_requests

    if tracked_requests <= 0 then
        M.results.error_rate = 0
        M.results.client_error_rate = 0
        M.results.http_error_rate = 0
        M.results.network_error_rate = 0
        M.results.conflict_rate = 0
        M.results.conflict_count = 0
        M.results.meta.success_rate = 0
        return
    end

    if M.results.status_distribution and type(M.results.status_distribution) == "table" and next(M.results.status_distribution) then
        local success_count = 0
        for status, count in pairs(M.results.status_distribution) do
            local status_num = tonumber(status)
            if status_num and status_num >= 200 and status_num < 300 and type(count) == "number" then
                success_count = success_count + count
            end
        end

        M.results.meta.success_rate = success_count / tracked_requests

        local total_http_errors = http_error_counts.count_4xx_total + http_error_counts.count_5xx_total
        local non_conflict_errors = http_error_counts.count_4xx_excluding_409 + http_error_counts.count_5xx_total
        M.results.error_rate = non_conflict_errors / tracked_requests
        M.results.client_error_rate = http_error_counts.count_4xx_excluding_409 / tracked_requests
        M.results.conflict_count = http_error_counts.count_409
        M.results.conflict_rate = http_error_counts.count_409 / tracked_requests
        M.results.http_error_rate = total_http_errors / tracked_requests
        M.results.server_error_rate = http_error_counts.count_5xx_total / tracked_requests
        M.results.status_code_counts = {
            count_400 = http_error_counts.count_400,
            count_404 = http_error_counts.count_404,
            count_409 = http_error_counts.count_409,
            count_422 = http_error_counts.count_422,
            count_500 = http_error_counts.count_500,
        }
    else
        M.results.error_rate = M.NULL
        M.results.client_error_rate = M.NULL
        M.results.conflict_rate = M.NULL
        M.results.conflict_count = M.NULL
        M.results.meta.success_rate = M.NULL
        local http_errors = (summary and summary.errors and summary.errors.status) or 0
        M.results.http_error_rate = http_errors / tracked_requests
        M.results.status_code_counts = {
            count_400 = M.NULL, count_404 = M.NULL, count_409 = M.NULL,
            count_422 = M.NULL, count_500 = M.NULL,
        }
    end

    if M.results.errors.connect == M.NULL then
        M.results.network_error_rate = M.NULL
    else
        local network_errors = (summary and summary.errors) and
            ((summary.errors.connect or 0) + (summary.errors.read or 0) +
             (summary.errors.write or 0) + (summary.errors.timeout or 0)) or 0
        M.results.network_error_rate = network_errors / M.results.total_requests
    end
end

    if error_tracker and type(error_tracker.get_all_threads_aggregated_summary) == "function" then
        local aggregated = error_tracker.get_all_threads_aggregated_summary()
        M.results.http_status = {}
        local http_status_total = 0

        for key, count in pairs(aggregated) do
            local status_code = key:match("^status_(.+)$")
            if status_code and count > 0 then
                M.results.http_status[status_code] = count
                http_status_total = http_status_total + count
            end
        end

        if http_status_total > 0 then
            M.results.meta.tracked_requests = http_status_total
            M.results.status_distribution = {}
            for status, count in pairs(M.results.http_status) do
                if (tonumber(status) or status == "other") and type(count) == "number" then
                    M.results.status_distribution[status] = count
                end
            end
        end
    elseif not M.results.http_status or not next(M.results.http_status) then
        if M.results.status_distribution and type(M.results.status_distribution) == "table" and next(M.results.status_distribution) then
            M.results.http_status = {}
            for status, count in pairs(M.results.status_distribution) do
                if tonumber(status) and type(count) == "number" then
                    M.results.http_status[status] = count
                end
            end
        end
    end

    if error_tracker then
        error_tracker.aggregate_from_summary(summary)
        local http_error_counts = aggregate_error_counts()
        if http_error_counts.total == 0 and summary and summary.errors and summary.errors.status then
            http_error_counts.total = summary.errors.status
        end
        error_tracker.set_http_error_counts(http_error_counts)
        M.results.errors_detail = error_tracker.get_summary()
        set_error_metrics(http_error_counts, summary)
    else
        local http_error_counts = aggregate_error_counts()
        set_error_metrics(http_error_counts, summary)
    end

    if error_tracker then
        local error_summary = error_tracker.get_summary()
        M.results.retries = error_summary.retry_count or M.retry_count or 0
    else
        M.results.retries = M.retry_count or 0
    end
end

function M.format_json()
    return M.encode_table_json(M.results)
end

function M.encode_table_json(tbl)
    if tbl == M.NULL then return "null" end

    local tbl_type = type(tbl)
    if tbl_type ~= "table" then
        if tbl_type == "string" then
            return '"' .. tbl:gsub('\\', '\\\\'):gsub('"', '\\"'):gsub('\n', '\\n') .. '"'
        elseif tbl_type == "boolean" then return tbl and "true" or "false"
        elseif tbl_type == "nil" then return "null"
        else return tostring(tbl) end
    end

    local parts = {}
    if #tbl > 0 then
        for _, v in ipairs(tbl) do table.insert(parts, M.encode_table_json(v)) end
        return "[" .. table.concat(parts, ",") .. "]"
    else
        for k, v in pairs(tbl) do
            table.insert(parts, '"' .. tostring(k) .. '":' .. M.encode_table_json(v))
        end
        return "{" .. table.concat(parts, ",") .. "}"
    end
end

function M.format_yaml()
    local lines = {}

    local function add_line(indent, key, value)
        local prefix = string.rep("  ", indent)
        local value_type = type(value)

        if value == M.NULL then table.insert(lines, string.format("%s%s: null", prefix, key))
        elseif value == nil then table.insert(lines, string.format("%s%s:", prefix, key))
        elseif value_type == "table" then
            table.insert(lines, string.format("%s%s:", prefix, key))
            for k, v in pairs(value) do add_line(indent + 1, k, v) end
        elseif value_type == "string" then table.insert(lines, string.format('%s%s: "%s"', prefix, key, value))
        elseif value_type == "boolean" then table.insert(lines, string.format("%s%s: %s", prefix, key, value and "true" or "false"))
        else table.insert(lines, string.format("%s%s: %s", prefix, key, tostring(value))) end
    end

    local function add_rate(key, value)
        if type(value) == "number" then add_line(0, key, string.format("%.4f", value))
        else add_line(0, key, value or "unknown") end
    end

    add_line(0, "scenario", M.results.scenario)
    add_line(0, "timestamp", M.results.timestamp)
    add_line(0, "duration_seconds", string.format("%.2f", M.results.duration_seconds))
    add_line(0, "total_requests", M.results.total_requests)
    add_line(0, "successful_requests", M.results.successful_requests)
    add_line(0, "failed_requests", M.results.failed_requests)
    add_rate("error_rate", M.results.error_rate)
    if M.results.http_error_rate == M.NULL then
        add_line(0, "http_error_rate", M.NULL)
    else
        add_line(0, "http_error_rate", string.format("%.4f", M.results.http_error_rate or 0))
    end
    if M.results.network_error_rate == M.NULL then
        add_line(0, "network_error_rate", M.NULL)
    else
        add_line(0, "network_error_rate", string.format("%.4f", M.results.network_error_rate or 0))
    end
    add_rate("client_error_rate", M.results.client_error_rate)
    add_line(0, "conflict_count", M.results.conflict_count or 0)
    add_rate("conflict_rate", M.results.conflict_rate)
    add_line(0, "status_code_counts")
    local scc = M.results.status_code_counts or {}
    add_line(1, "count_400", scc.count_400 or 0)
    add_line(1, "count_404", scc.count_404 or 0)
    add_line(1, "count_409", scc.count_409 or 0)
    add_line(1, "count_422", scc.count_422 or 0)
    add_line(1, "count_500", scc.count_500 or 0)

    add_line(0, "rps")
    add_line(1, "target", M.results.rps.target)
    add_line(1, "actual", string.format("%.1f", M.results.rps.actual))

    add_line(0, "latency")
    local latency_fields = {"min_ms", "max_ms", "mean_ms", "stddev_ms"}
    for _, field in ipairs(latency_fields) do
        add_line(1, field, string.format("%.2f", M.results.latency[field]))
    end
    local percentile_fields = {"p50_ms", "p75_ms", "p90_ms", "p95_ms", "p99_ms", "p999_ms"}
    for _, field in ipairs(percentile_fields) do
        add_line(1, field, M.results.latency[field])
    end

    add_line(0, "payload")
    add_line(1, "variant", M.results.payload.variant or "unknown")
    add_line(1, "estimated_size_bytes", M.results.payload.estimated_size_bytes or 0)

    add_line(0, "load_profile")
    add_line(1, "profile", M.results.load_profile.profile or "unknown")
    add_line(1, "target_rps", M.results.load_profile.target_rps or 0)

    add_line(0, "errors")
    -- Handle M.NULL for thread-local mode
    add_line(1, "connect", M.results.errors.connect)
    add_line(1, "read", M.results.errors.read)
    add_line(1, "write", M.results.errors.write)
    add_line(1, "timeout", M.results.errors.timeout)

    if type(M.results.errors.status) == "table" then
        add_line(1, "status")
        add_line(2, "4xx", M.results.errors.status["4xx"] or 0)
        add_line(2, "5xx", M.results.errors.status["5xx"] or 0)
    elseif type(M.results.errors.status) == "number" then add_line(1, "status", M.results.errors.status)
    else add_line(1, "status", 0) end

    if M.results.errors.status_total and M.results.errors.status_total ~= M.NULL then
        add_line(1, "status_total", M.results.errors.status_total)
    elseif M.results.errors.status_total == M.NULL then
        add_line(1, "status_total", M.NULL)
    end

    return table.concat(lines, "\n")
end

function M.format_text()
    local lines = {}
    local scenario_name = M.results.scenario and M.results.scenario.name or "unknown"

    table.insert(lines, "")
    table.insert(lines, "====================================")
    table.insert(lines, string.format("Benchmark Results: %s", scenario_name))
    table.insert(lines, "====================================")
    table.insert(lines, string.format("Timestamp: %s", M.results.timestamp))
    table.insert(lines, string.format("Duration: %.2f seconds", M.results.duration_seconds))
    table.insert(lines, "")

    local function format_rate(label, value)
        if value == M.NULL then
            return string.format("%s %s", label, "N/A")
        elseif type(value) == "number" then
            return string.format("%s %.2f%%", label, value * 100)
        else
            return string.format("%s %s", label, value or "unknown")
        end
    end

    local function format_count(label, value)
        if value == M.NULL then
            return string.format("%s %s", label, "N/A")
        elseif type(value) == "number" then
            return string.format("%s %d", label, value)
        else
            return string.format("%s %s", label, tostring(value or 0))
        end
    end

    table.insert(lines, "--- Request Summary ---")
    table.insert(lines, string.format("Total requests:      %d", M.results.total_requests))
    table.insert(lines, string.format("Successful requests: %d", M.results.successful_requests))
    table.insert(lines, string.format("Failed requests:     %d", M.results.failed_requests))
    if M.results.http_error_rate == M.NULL then
        table.insert(lines, "HTTP error rate:     N/A (4xx/5xx only)")
    else
        table.insert(lines, string.format("HTTP error rate:     %.2f%% (4xx/5xx only)", (M.results.http_error_rate or 0) * 100))
    end
    if M.results.network_error_rate == M.NULL then
        table.insert(lines, "Network error rate:  N/A (socket errors)")
    else
        table.insert(lines, string.format("Network error rate:  %.2f%% (socket errors)", (M.results.network_error_rate or 0) * 100))
    end
    table.insert(lines, format_rate("Error rate (excl.409):", M.results.error_rate))
    table.insert(lines, format_rate("Client error rate:  ", M.results.client_error_rate))
    table.insert(lines, format_count("Conflict count:     ", M.results.conflict_count))
    table.insert(lines, format_rate("Conflict rate:      ", M.results.conflict_rate))
    table.insert(lines, "")
    table.insert(lines, "--- Status Code Counts ---")
    local scc = M.results.status_code_counts or {}
    local status_labels = {
        {"400 Bad Request", "count_400"},
        {"404 Not Found", "count_404"},
        {"409 Conflict", "count_409"},
        {"422 Unprocessable", "count_422"},
        {"500 Server Error", "count_500"}
    }
    for _, item in ipairs(status_labels) do
        table.insert(lines, format_count(string.format("%-20s", item[1] .. ":"), scc[item[2]]))
    end
    table.insert(lines, "")

    table.insert(lines, "--- Throughput ---")
    table.insert(lines, string.format("Target RPS:  %d", M.results.rps.target))
    table.insert(lines, string.format("Actual RPS:  %.1f", M.results.rps.actual))
    table.insert(lines, "")

    local function format_percentile(value)
        return (value ~= M.NULL and type(value) == "number") and string.format("%.2f", value) or "N/A"
    end

    table.insert(lines, "--- Latency (ms) ---")
    local latency_stats = {
        {"Min", M.results.latency.min_ms},
        {"Max", M.results.latency.max_ms},
        {"Mean", M.results.latency.mean_ms},
        {"StdDev", M.results.latency.stddev_ms}
    }
    for _, stat in ipairs(latency_stats) do
        table.insert(lines, string.format("%-7s %.2f", stat[1] .. ":", stat[2]))
    end
    table.insert(lines, "")
    table.insert(lines, "Percentiles:")
    local percentiles = {
        {"p50", M.results.latency.p50_ms},
        {"p75", M.results.latency.p75_ms},
        {"p90", M.results.latency.p90_ms},
        {"p95", M.results.latency.p95_ms},
        {"p99", M.results.latency.p99_ms},
        {"p99.9", M.results.latency.p999_ms}
    }
    for _, p in ipairs(percentiles) do
        table.insert(lines, string.format("  %-6s %s", p[1] .. ":", format_percentile(p[2])))
    end
    table.insert(lines, "")

    table.insert(lines, "--- Payload ---")
    table.insert(lines, string.format("Variant:        %s", M.results.payload.variant or "unknown"))
    table.insert(lines, string.format("Estimated size: %d bytes", M.results.payload.estimated_size_bytes or 0))
    table.insert(lines, "")

    table.insert(lines, "--- Load Profile ---")
    table.insert(lines, string.format("Profile:    %s", M.results.load_profile.profile or "unknown"))
    table.insert(lines, string.format("Target RPS: %d", M.results.load_profile.target_rps or 0))
    table.insert(lines, "")

    table.insert(lines, "--- Errors Breakdown ---")
    local error_types = {"Connect", "Read", "Write", "Timeout"}
    if M.results.errors.connect == M.NULL then
        for _, err_type in ipairs(error_types) do
            table.insert(lines, string.format("%-8s N/A", err_type .. ":"))
        end
    else
        for _, err_type in ipairs(error_types) do
            local key = string.lower(err_type)
            table.insert(lines, string.format("%-8s %d", err_type .. ":", M.results.errors[key]))
        end
    end

    if type(M.results.errors.status) == "table" then
        local status_errors_total = 0
        for _, count in pairs(M.results.errors.status) do
            if type(count) == "number" then status_errors_total = status_errors_total + count end
        end
        table.insert(lines, string.format("Status (total): %d", status_errors_total))
        table.insert(lines, string.format("  4xx: %d", M.results.errors.status["4xx"] or 0))
        table.insert(lines, string.format("  5xx: %d", M.results.errors.status["5xx"] or 0))
    elseif type(M.results.errors.status) == "number" then
        table.insert(lines, string.format("Status: %d", M.results.errors.status))
    else
        table.insert(lines, string.format("Status: %s", tostring(M.results.errors.status or 0)))
    end

    if M.results.errors.status_total and M.results.errors.status_total ~= M.NULL and M.results.errors.status_total > 0 then
        table.insert(lines, string.format("Status (from wrk): %d", M.results.errors.status_total))
    elseif M.results.errors.status_total == M.NULL then
        table.insert(lines, "Status (from wrk): N/A")
    end

    table.insert(lines, "====================================")

    return table.concat(lines, "\n")
end

function M.print_results()
    local output = M.config.output_format == "json" and M.format_json()
                or M.config.output_format == "yaml" and M.format_yaml()
                or M.format_text()
    io.write(output, "\n")
end

function M.save_results(filepath)
    filepath = filepath or M.config.output_file
    if not filepath then return end

    local output = filepath:match("%.json$") and M.format_json()
                or filepath:match("%.ya?ml$") and M.format_yaml()
                or M.format_text()

    local file = io.open(filepath, "w")
    if file then
        file:write(output, "\n")
        file:close()
        io.write(string.format("[result_collector] Results saved to: %s\n", filepath))
    else
        io.stderr:write(string.format("[result_collector] Failed to save results to: %s\n", filepath))
    end
end

function M.get_results()
    return M.results
end

function M.format_extended_json()
    local result = {
        scenario = M.results.scenario,
        execution = M.results.execution,
        latency = {
            min_us = M.results.latency.min_us,
            max_us = M.results.latency.max_us,
            mean_us = M.results.latency.mean_us,
            stdev_us = M.results.latency.stdev_us,
            percentiles = M.results.latency.percentiles
        },
        throughput = M.results.throughput,
        errors = M.results.errors,
        status_distribution = M.results.status_distribution
    }
    return M.encode_table_json(result)
end

function M.save_extended_results(filepath)
    if not filepath then return end

    local file = io.open(filepath, "w")
    if file then
        file:write(M.format_extended_json(), "\n")
        file:close()
        io.write(string.format("[result_collector] Extended results saved to: %s\n", filepath))
    else
        io.stderr:write(string.format("[result_collector] Failed to save extended results to: %s\n", filepath))
    end
end

return M
