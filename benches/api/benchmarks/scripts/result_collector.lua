-- Result Collector Module for wrk benchmarks
-- benches/api/benchmarks/scripts/result_collector.lua
--
-- Collects and formats benchmark results including:
--   - Latency percentiles (p50, p95, p99)
--   - Error rates
--   - RPS (actual vs target)
--   - Payload size information
--
-- Usage:
--   local result_collector = require("result_collector")
--   result_collector.init({
--       scenario_name = "my_benchmark",
--       output_format = "json"  -- or "yaml", "text"
--   })
--
--   -- In response():
--   result_collector.record_response(status, latency_ms)
--
--   -- In done():
--   result_collector.finalize(summary, latency, requests)
--   result_collector.print_results()
--   result_collector.save_results("/path/to/output.json")

local M = {}

-- Attempt to load cache_metrics module (optional dependency)
local cache_metrics_ok, cache_metrics = pcall(require, "cache_metrics")
if not cache_metrics_ok then
    cache_metrics = nil
end

-- Attempt to load error_tracker module (optional dependency)
local error_tracker_ok, error_tracker = pcall(require, "error_tracker")
if not error_tracker_ok then
    error_tracker = nil
end

-- Configuration
M.config = {
    scenario_name = "benchmark",
    output_format = "json",  -- json, yaml, text
    output_file = nil,       -- Optional output file path
    include_raw_latencies = false  -- Whether to include raw latency samples
}

-- Result data (extended format for profiling integration)
M.results = {
    -- Scenario metadata
    scenario = {
        name = nil,
        storage_mode = nil,
        cache_mode = nil,
        load_pattern = nil,
        contention_level = nil
    },
    -- Execution context
    execution = {
        timestamp = nil,
        duration_seconds = 0,
        threads = 0,
        connections = 0
    },
    -- Legacy fields for backward compatibility
    timestamp = nil,
    duration_seconds = 0,
    total_requests = 0,
    successful_requests = 0,
    failed_requests = 0,
    -- Error rates (4xx/5xx only, excluding network errors)
    error_rate = 0,
    http_error_rate = 0,
    network_error_rate = 0,
    -- Conflict metrics (409 responses)
    conflict_rate = 0,
    conflict_count = 0,
    -- Individual status code counts
    status_code_counts = {
        count_400 = 0,
        count_404 = 0,
        count_409 = 0,
        count_422 = 0,
        count_500 = 0,
    },
    rps = {
        target = 0,
        actual = 0
    },
    -- Latency metrics (microseconds for precision, milliseconds for display)
    latency = {
        min_us = 0,
        max_us = 0,
        mean_us = 0,
        stdev_us = 0,
        -- Also store milliseconds for backward compatibility
        min_ms = 0,
        max_ms = 0,
        mean_ms = 0,
        stddev_ms = 0,
        -- Percentiles
        percentiles = {
            p50 = 0,
            p75 = 0,
            p90 = 0,
            p95 = 0,
            p99 = 0,
            p99_9 = 0
        },
        -- Legacy percentile fields
        p50_ms = 0,
        p75_ms = 0,
        p90_ms = 0,
        p95_ms = 0,
        p99_ms = 0,
        p999_ms = 0
    },
    -- Throughput metrics
    throughput = {
        requests_total = 0,
        requests_per_second = 0,
        bytes_total = 0,
        bytes_per_second = 0
    },
    payload = {
        variant = nil,
        estimated_size_bytes = 0
    },
    load_profile = {
        profile = nil,
        target_rps = 0
    },
    -- Status code distribution
    status_distribution = {},
    status_codes = {},
    -- Error breakdown
    errors = {
        connect = 0,
        read = 0,
        write = 0,
        timeout = 0,
        status = {
            ["4xx"] = 0,
            ["5xx"] = 0
        }
    },
    -- Cache metrics (populated from cache_metrics module)
    cache = nil,
    -- Error tracking detail (populated from error_tracker module)
    errors_detail = nil
}

-- Internal state
M.response_count = 0
M.error_count = 0
M.status_counts = {}
M.start_time = nil
M.current_endpoint = nil  -- Track current endpoint for cache metrics

-- Initialize the result collector
-- @param options table Configuration options
function M.init(options)
    options = options or {}

    for key, value in pairs(options) do
        if M.config[key] ~= nil then
            M.config[key] = value
        end
    end

    -- Initialize timestamps
    local timestamp = os.date("!%Y-%m-%dT%H:%M:%SZ")
    M.results.scenario.name = M.config.scenario_name
    M.results.execution.timestamp = timestamp
    M.results.timestamp = timestamp  -- Legacy
    M.start_time = os.time()

    -- Initialize execution context from environment variables
    M.results.execution.threads = tonumber(os.getenv("THREADS")) or 0
    M.results.execution.connections = tonumber(os.getenv("CONNECTIONS")) or 0

    -- Automatically load scenario metadata from environment variables
    -- This ensures storage_mode, cache_mode, load_pattern, contention_level are populated
    M.load_scenario_from_env()

    -- Also load scenario name from environment if not provided in options
    local env_scenario_name = os.getenv("SCENARIO_NAME")
    if env_scenario_name and env_scenario_name ~= "" and M.results.scenario.name == "benchmark" then
        M.results.scenario.name = env_scenario_name
    end

    -- Reset counters
    M.response_count = 0
    M.error_count = 0
    M.status_counts = {}

    -- Initialize cache metrics module if available
    if cache_metrics then
        cache_metrics.init()
    end

    -- Initialize error tracker module if available
    if error_tracker then
        error_tracker.init()
    end
end

-- Set scenario metadata from environment or explicit configuration
-- @param metadata table Scenario metadata
function M.set_scenario_metadata(metadata)
    if metadata then
        M.results.scenario.name = metadata.name or M.results.scenario.name
        M.results.scenario.storage_mode = metadata.storage_mode
        M.results.scenario.cache_mode = metadata.cache_mode
        M.results.scenario.load_pattern = metadata.load_pattern
        M.results.scenario.contention_level = metadata.contention_level
    end
end

-- Load scenario metadata from environment variables
function M.load_scenario_from_env()
    M.results.scenario.storage_mode = os.getenv("STORAGE_MODE")
    M.results.scenario.cache_mode = os.getenv("CACHE_MODE")
    M.results.scenario.load_pattern = os.getenv("LOAD_PATTERN")
    M.results.scenario.contention_level = os.getenv("CONTENTION_LEVEL")
end

-- Record a response (called from response callback)
-- @param status number HTTP status code
-- @param latency_us number Response latency in microseconds (optional, wrk does not provide this directly in response())
-- @param headers table HTTP response headers (optional, for cache detection)
-- @param endpoint string The endpoint path (optional, for per-endpoint metrics)
-- NOTE: This function is called from worker threads, but wrk uses separate Lua
-- interpreters per thread. Therefore, M.status_counts accumulated here is
-- thread-local and will NOT be available in done() (which runs in main thread).
-- For accurate error counts, use wrk's summary.errors in finalize() instead.
-- Status code distribution tracking is best-effort and may be incomplete.
function M.record_response(status, latency_us, headers, endpoint)
    M.response_count = M.response_count + 1

    -- Track status codes (thread-local, may not aggregate correctly)
    local status_key = tostring(status)
    M.status_counts[status_key] = (M.status_counts[status_key] or 0) + 1

    -- Track errors (thread-local)
    if status >= 400 then
        M.error_count = M.error_count + 1
    end

    -- Track errors with error_tracker if available
    if error_tracker then
        error_tracker.track_response(status, nil)
    end

    -- Track cache metrics if available and enabled
    if cache_metrics and cache_metrics.state.enabled then
        -- Use endpoint parameter if provided, fall back to current_endpoint, then "unknown"
        local effective_endpoint = endpoint or M.current_endpoint or "unknown"
        local is_cache_hit = M.detect_cache_hit(headers)
        -- Note: wrk's response() callback does not provide latency directly;
        -- latency statistics are only available in done() via the latency object.
        -- We pass 0 here; actual latency is tracked by wrk internally.
        cache_metrics.track(effective_endpoint, is_cache_hit, latency_us or 0)
    end
end

-- Set the current endpoint for cache metrics tracking
-- @param endpoint string The endpoint path
-- NOTE: This function is deprecated. Prefer passing endpoint directly to record_response().
-- It remains available for backwards compatibility and for cases where
-- request/response order might differ (though wrk processes them sequentially per thread).
function M.set_current_endpoint(endpoint)
    M.current_endpoint = endpoint
end

-- Detect cache hit from HTTP response headers
-- @param headers table HTTP response headers
-- @return boolean True if response was served from cache
function M.detect_cache_hit(headers)
    if not headers then return false end

    -- X-Cache: HIT (common CDN/proxy header)
    local x_cache = headers["X-Cache"] or headers["x-cache"]
    if x_cache and string.find(string.lower(x_cache), "hit") then
        return true
    end

    -- X-Cache-Hit: true/1 (some cache implementations)
    local x_cache_hit = headers["X-Cache-Hit"] or headers["x-cache-hit"]
    if x_cache_hit and (x_cache_hit == "true" or x_cache_hit == "1") then
        return true
    end

    -- Age header present with value > 0 indicates cached response
    local age = headers["Age"] or headers["age"]
    if age then
        local age_value = tonumber(age)
        if age_value and age_value > 0 then
            return true
        end
    end

    -- X-Cache-Status: HIT (nginx)
    local x_cache_status = headers["X-Cache-Status"] or headers["x-cache-status"]
    if x_cache_status and string.find(string.lower(x_cache_status), "hit") then
        return true
    end

    -- CF-Cache-Status: HIT (Cloudflare)
    local cf_cache_status = headers["CF-Cache-Status"] or headers["cf-cache-status"]
    if cf_cache_status and string.find(string.lower(cf_cache_status), "hit") then
        return true
    end

    return false
end

-- Set load profile metadata
-- @param profile_metadata table From load_profile.get_profile_metadata()
function M.set_load_profile(profile_metadata)
    if profile_metadata then
        M.results.load_profile = profile_metadata
        M.results.rps.target = profile_metadata.target_rps or 0
    end
end

-- Set payload metadata
-- @param payload_metadata table From payload_generator.get_metadata()
function M.set_payload(payload_metadata)
    if payload_metadata then
        M.results.payload = payload_metadata
    end
end

-- Finalize results (called from done callback)
-- @param summary table wrk summary object
-- @param latency table wrk latency object
-- @param requests table wrk requests object
-- NOTE: wrk runs multiple threads with separate Lua interpreters.
-- M.status_counts collected in response() callbacks are thread-local and
-- typically empty in done() (main thread). Use summary.errors for accurate
-- error counts. Detailed status code distribution is not available via wrk's API.
function M.finalize(summary, latency, requests)
    if not summary then
        -- Fallback to internal counters (unlikely to be accurate due to thread isolation)
        M.results.total_requests = M.response_count
        M.results.failed_requests = M.error_count
        M.results.successful_requests = M.response_count - M.error_count
        M.results.duration_seconds = os.time() - (M.start_time or os.time())
        M.results.execution.duration_seconds = M.results.duration_seconds
        return
    end

    -- Use wrk's summary for accurate, thread-aggregated counts
    local duration_seconds = summary.duration / 1000000  -- Convert from microseconds
    M.results.duration_seconds = duration_seconds
    M.results.execution.duration_seconds = duration_seconds
    M.results.total_requests = summary.requests
    M.results.successful_requests = summary.requests - (summary.errors.connect + summary.errors.read +
                                                         summary.errors.write + summary.errors.timeout +
                                                         summary.errors.status)
    M.results.failed_requests = summary.requests - M.results.successful_requests

    -- Error rates will be calculated after categorizing HTTP errors
    -- See the error_tracker integration section below

    -- Calculate actual RPS
    if duration_seconds > 0 then
        M.results.rps.actual = M.results.total_requests / duration_seconds
    end

    -- Update throughput metrics (extended format)
    M.results.throughput.requests_total = summary.requests
    M.results.throughput.requests_per_second = M.results.rps.actual
    M.results.throughput.bytes_total = summary.bytes or 0
    if duration_seconds > 0 then
        M.results.throughput.bytes_per_second = (summary.bytes or 0) / duration_seconds
    end

    -- Record errors breakdown from summary (thread-aggregated, accurate)
    -- Extended format with status code categories
    M.results.errors = {
        connect = summary.errors.connect,
        read = summary.errors.read,
        write = summary.errors.write,
        timeout = summary.errors.timeout,
        status = {
            ["4xx"] = 0,  -- Cannot distinguish from wrk summary
            ["5xx"] = 0   -- Cannot distinguish from wrk summary
        }
    }
    -- Store total status errors for reference
    M.results.errors.status_total = summary.errors.status

    -- Record status code distribution
    -- NOTE: M.status_counts is thread-local and may be incomplete or empty in done().
    -- wrk does not provide detailed status code distribution in its summary.
    -- We record what's available but this is for reference only.
    -- For accurate success/error counts, use M.results.successful_requests and M.results.failed_requests.
    if next(M.status_counts) then
        M.results.status_codes = M.status_counts
        M.results.status_distribution = M.status_counts

        -- Categorize status codes for extended format
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
        -- No thread-local status distribution available
        -- Keep status_distribution empty to trigger fallback logic later
        -- NOTE: We intentionally do NOT insert dummy values here.
        -- The http_error_counts calculation below will use summary.errors.status as fallback.
        M.results.status_codes = {
            ["2xx"] = M.results.successful_requests,
            ["errors"] = M.results.failed_requests,
            ["_note"] = "Detailed status code distribution unavailable due to wrk thread isolation"
        }
        M.results.status_distribution = {}
    end

    -- Extract latency percentiles from wrk's latency object
    if latency then
        -- Store in microseconds (extended format)
        M.results.latency.min_us = latency.min
        M.results.latency.max_us = latency.max
        M.results.latency.mean_us = latency.mean
        M.results.latency.stdev_us = latency.stdev

        -- Store in milliseconds (legacy format)
        M.results.latency.min_ms = latency.min / 1000
        M.results.latency.max_ms = latency.max / 1000
        M.results.latency.mean_ms = latency.mean / 1000
        M.results.latency.stddev_ms = latency.stdev / 1000

        -- Percentiles in microseconds (extended format)
        local p50 = latency:percentile(50)
        local p75 = latency:percentile(75)
        local p90 = latency:percentile(90)
        local p95 = latency:percentile(95)
        local p99 = latency:percentile(99)
        local p999 = latency:percentile(99.9)

        M.results.latency.percentiles = {
            p50 = p50,
            p75 = p75,
            p90 = p90,
            p95 = p95,
            p99 = p99,
            p99_9 = p999
        }

        -- Percentiles in milliseconds (legacy format)
        M.results.latency.p50_ms = p50 / 1000
        M.results.latency.p75_ms = p75 / 1000
        M.results.latency.p90_ms = p90 / 1000
        M.results.latency.p95_ms = p95 / 1000
        M.results.latency.p99_ms = p99 / 1000
        M.results.latency.p999_ms = p999 / 1000
    end

    -- Collect cache metrics if available
    if cache_metrics then
        M.results.cache = cache_metrics.get_summary()
    end

    -- Collect error tracking metrics if available
    -- Use aggregate_from_summary to get accurate thread-aggregated error counts
    if error_tracker then
        -- Aggregate from wrk's summary (thread-safe, accurate)
        error_tracker.aggregate_from_summary(summary)

        -- Calculate HTTP error counts by status code category
        -- NOTE: wrk's summary.errors.status includes non-2xx (including 3xx).
        -- For accurate 4xx/5xx counts, we need status_distribution from thread-local data.
        -- Since thread-local data may be incomplete in multi-thread mode, we use
        -- summary.errors.status as an upper bound for HTTP errors.
        local http_error_counts = {
            total = 0,
            count_400 = 0,
            count_404 = 0,
            count_409 = 0,
            count_422 = 0,
            count_500 = 0,
        }

        -- Try to use thread-local status_distribution if available
        if M.results.status_distribution and next(M.results.status_distribution) then
            for status, count in pairs(M.results.status_distribution) do
                local status_num = tonumber(status)
                if status_num and status_num >= 400 and type(count) == "number" then
                    http_error_counts.total = http_error_counts.total + count
                    -- Track individual status codes
                    if status_num == 400 then
                        http_error_counts.count_400 = count
                    elseif status_num == 404 then
                        http_error_counts.count_404 = count
                    elseif status_num == 409 then
                        http_error_counts.count_409 = count
                    elseif status_num == 422 then
                        http_error_counts.count_422 = count
                    elseif status_num == 500 then
                        http_error_counts.count_500 = count
                    end
                end
            end
        else
            -- Fallback: use wrk's summary.errors.status as total HTTP errors
            -- NOTE: This may include 3xx responses, so it's an approximation
            if summary and summary.errors and summary.errors.status then
                http_error_counts.total = summary.errors.status
            end
        end

        error_tracker.set_http_error_counts(http_error_counts)

        -- Extract metrics from error_tracker
        local error_summary = error_tracker.get_summary()
        M.results.errors_detail = error_summary

        -- Update error rates (4xx/5xx only)
        M.results.http_error_rate = error_summary.http_error_rate or 0
        M.results.network_error_rate = error_summary.network_error_rate or 0
        M.results.error_rate = error_summary.http_error_rate or 0  -- Legacy: now uses HTTP errors only

        -- Update conflict metrics
        M.results.conflict_count = error_summary.conflict_count or 0
        M.results.conflict_rate = error_summary.conflict_rate or 0

        -- Update individual status code counts
        M.results.status_code_counts = {
            count_400 = http_error_counts.count_400,
            count_404 = http_error_counts.count_404,
            count_409 = http_error_counts.count_409,
            count_422 = http_error_counts.count_422,
            count_500 = http_error_counts.count_500,
        }
    else
        -- Fallback: calculate error_rate without error_tracker
        -- NOTE: Without error_tracker, we use summary.errors.status as HTTP error count.
        -- This may include 3xx responses (wrk counts non-2xx as "status" errors).
        -- For accurate 4xx/5xx counts, use error_tracker with status distribution.
        if M.results.total_requests > 0 then
            -- Use summary.errors.status as HTTP error approximation (may include 3xx)
            local http_errors = (summary and summary.errors and summary.errors.status) or 0
            M.results.error_rate = http_errors / M.results.total_requests
            M.results.http_error_rate = M.results.error_rate
            -- Calculate network error rate separately
            local network_errors = (summary and summary.errors) and
                ((summary.errors.connect or 0) + (summary.errors.read or 0) +
                 (summary.errors.write or 0) + (summary.errors.timeout or 0)) or 0
            M.results.network_error_rate = network_errors / M.results.total_requests
        end
    end
end

-- Format results as JSON
-- @return string JSON formatted results
function M.format_json()
    local function encode_value(v)
        if type(v) == "string" then
            return '"' .. v:gsub('\\', '\\\\'):gsub('"', '\\"'):gsub('\n', '\\n') .. '"'
        elseif type(v) == "boolean" then
            return v and "true" or "false"
        elseif type(v) == "nil" then
            return "null"
        elseif type(v) == "table" then
            return M.encode_table_json(v)
        else
            return tostring(v)
        end
    end

    return M.encode_table_json(M.results)
end

-- Helper to encode table as JSON
function M.encode_table_json(tbl)
    if type(tbl) ~= "table" then
        if type(tbl) == "string" then
            return '"' .. tbl:gsub('\\', '\\\\'):gsub('"', '\\"'):gsub('\n', '\\n') .. '"'
        elseif type(tbl) == "boolean" then
            return tbl and "true" or "false"
        elseif type(tbl) == "nil" then
            return "null"
        else
            return tostring(tbl)
        end
    end

    -- Check if array
    local is_array = #tbl > 0
    local parts = {}

    if is_array then
        for _, v in ipairs(tbl) do
            table.insert(parts, M.encode_table_json(v))
        end
        return "[" .. table.concat(parts, ",") .. "]"
    else
        for k, v in pairs(tbl) do
            table.insert(parts, '"' .. tostring(k) .. '":' .. M.encode_table_json(v))
        end
        return "{" .. table.concat(parts, ",") .. "}"
    end
end

-- Format results as YAML
-- @return string YAML formatted results
function M.format_yaml()
    local lines = {}

    local function add_line(indent, key, value)
        local prefix = string.rep("  ", indent)
        if value == nil then
            table.insert(lines, string.format("%s%s:", prefix, key))
        elseif type(value) == "table" then
            table.insert(lines, string.format("%s%s:", prefix, key))
            for k, v in pairs(value) do
                add_line(indent + 1, k, v)
            end
        elseif type(value) == "string" then
            table.insert(lines, string.format('%s%s: "%s"', prefix, key, value))
        elseif type(value) == "boolean" then
            table.insert(lines, string.format("%s%s: %s", prefix, key, value and "true" or "false"))
        else
            table.insert(lines, string.format("%s%s: %s", prefix, key, tostring(value)))
        end
    end

    add_line(0, "scenario", M.results.scenario)
    add_line(0, "timestamp", M.results.timestamp)
    add_line(0, "duration_seconds", string.format("%.2f", M.results.duration_seconds))
    add_line(0, "total_requests", M.results.total_requests)
    add_line(0, "successful_requests", M.results.successful_requests)
    add_line(0, "failed_requests", M.results.failed_requests)
    add_line(0, "error_rate", string.format("%.4f", M.results.error_rate))
    add_line(0, "http_error_rate", string.format("%.4f", M.results.http_error_rate or 0))
    add_line(0, "network_error_rate", string.format("%.4f", M.results.network_error_rate or 0))
    add_line(0, "conflict_count", M.results.conflict_count or 0)
    add_line(0, "conflict_rate", string.format("%.4f", M.results.conflict_rate or 0))
    add_line(0, "status_code_counts")
    add_line(1, "count_400", M.results.status_code_counts and M.results.status_code_counts.count_400 or 0)
    add_line(1, "count_404", M.results.status_code_counts and M.results.status_code_counts.count_404 or 0)
    add_line(1, "count_409", M.results.status_code_counts and M.results.status_code_counts.count_409 or 0)
    add_line(1, "count_422", M.results.status_code_counts and M.results.status_code_counts.count_422 or 0)
    add_line(1, "count_500", M.results.status_code_counts and M.results.status_code_counts.count_500 or 0)

    add_line(0, "rps")
    add_line(1, "target", M.results.rps.target)
    add_line(1, "actual", string.format("%.1f", M.results.rps.actual))

    add_line(0, "latency")
    add_line(1, "min_ms", string.format("%.2f", M.results.latency.min_ms))
    add_line(1, "max_ms", string.format("%.2f", M.results.latency.max_ms))
    add_line(1, "mean_ms", string.format("%.2f", M.results.latency.mean_ms))
    add_line(1, "stddev_ms", string.format("%.2f", M.results.latency.stddev_ms))
    add_line(1, "p50_ms", string.format("%.2f", M.results.latency.p50_ms))
    add_line(1, "p75_ms", string.format("%.2f", M.results.latency.p75_ms))
    add_line(1, "p90_ms", string.format("%.2f", M.results.latency.p90_ms))
    add_line(1, "p95_ms", string.format("%.2f", M.results.latency.p95_ms))
    add_line(1, "p99_ms", string.format("%.2f", M.results.latency.p99_ms))
    add_line(1, "p999_ms", string.format("%.2f", M.results.latency.p999_ms))

    add_line(0, "payload")
    add_line(1, "variant", M.results.payload.variant or "unknown")
    add_line(1, "estimated_size_bytes", M.results.payload.estimated_size_bytes or 0)

    add_line(0, "load_profile")
    add_line(1, "profile", M.results.load_profile.profile or "unknown")
    add_line(1, "target_rps", M.results.load_profile.target_rps or 0)

    add_line(0, "errors")
    add_line(1, "connect", M.results.errors.connect)
    add_line(1, "read", M.results.errors.read)
    add_line(1, "write", M.results.errors.write)
    add_line(1, "timeout", M.results.errors.timeout)

    -- Handle errors.status which is a table containing 4xx and 5xx counts
    if type(M.results.errors.status) == "table" then
        add_line(1, "status")
        add_line(2, "4xx", M.results.errors.status["4xx"] or 0)
        add_line(2, "5xx", M.results.errors.status["5xx"] or 0)
    elseif type(M.results.errors.status) == "number" then
        add_line(1, "status", M.results.errors.status)
    else
        add_line(1, "status", 0)
    end

    if M.results.errors.status_total then
        add_line(1, "status_total", M.results.errors.status_total)
    end

    return table.concat(lines, "\n")
end

-- Format results as human-readable text
-- @return string Text formatted results
function M.format_text()
    local lines = {}

    -- Extract scenario name from scenario table
    local scenario_name = M.results.scenario and M.results.scenario.name or "unknown"

    table.insert(lines, "")
    table.insert(lines, "====================================")
    table.insert(lines, string.format("Benchmark Results: %s", scenario_name))
    table.insert(lines, "====================================")
    table.insert(lines, string.format("Timestamp: %s", M.results.timestamp))
    table.insert(lines, string.format("Duration: %.2f seconds", M.results.duration_seconds))
    table.insert(lines, "")

    table.insert(lines, "--- Request Summary ---")
    table.insert(lines, string.format("Total requests:      %d", M.results.total_requests))
    table.insert(lines, string.format("Successful requests: %d", M.results.successful_requests))
    table.insert(lines, string.format("Failed requests:     %d", M.results.failed_requests))
    table.insert(lines, string.format("HTTP error rate:     %.2f%% (4xx/5xx only)", (M.results.http_error_rate or 0) * 100))
    table.insert(lines, string.format("Network error rate:  %.2f%% (socket errors)", (M.results.network_error_rate or 0) * 100))
    table.insert(lines, string.format("Conflict count:      %d", M.results.conflict_count or 0))
    table.insert(lines, string.format("Conflict rate:       %.2f%%", (M.results.conflict_rate or 0) * 100))
    table.insert(lines, "")
    table.insert(lines, "--- Status Code Counts ---")
    local status_counts = M.results.status_code_counts or {}
    table.insert(lines, string.format("400 Bad Request:     %d", status_counts.count_400 or 0))
    table.insert(lines, string.format("404 Not Found:       %d", status_counts.count_404 or 0))
    table.insert(lines, string.format("409 Conflict:        %d", status_counts.count_409 or 0))
    table.insert(lines, string.format("422 Unprocessable:   %d", status_counts.count_422 or 0))
    table.insert(lines, string.format("500 Server Error:    %d", status_counts.count_500 or 0))
    table.insert(lines, "")

    table.insert(lines, "--- Throughput ---")
    table.insert(lines, string.format("Target RPS:  %d", M.results.rps.target))
    table.insert(lines, string.format("Actual RPS:  %.1f", M.results.rps.actual))
    table.insert(lines, "")

    table.insert(lines, "--- Latency (ms) ---")
    table.insert(lines, string.format("Min:    %.2f", M.results.latency.min_ms))
    table.insert(lines, string.format("Max:    %.2f", M.results.latency.max_ms))
    table.insert(lines, string.format("Mean:   %.2f", M.results.latency.mean_ms))
    table.insert(lines, string.format("StdDev: %.2f", M.results.latency.stddev_ms))
    table.insert(lines, "")
    table.insert(lines, "Percentiles:")
    table.insert(lines, string.format("  p50:   %.2f", M.results.latency.p50_ms))
    table.insert(lines, string.format("  p75:   %.2f", M.results.latency.p75_ms))
    table.insert(lines, string.format("  p90:   %.2f", M.results.latency.p90_ms))
    table.insert(lines, string.format("  p95:   %.2f", M.results.latency.p95_ms))
    table.insert(lines, string.format("  p99:   %.2f", M.results.latency.p99_ms))
    table.insert(lines, string.format("  p99.9: %.2f", M.results.latency.p999_ms))
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
    table.insert(lines, string.format("Connect: %d", M.results.errors.connect))
    table.insert(lines, string.format("Read:    %d", M.results.errors.read))
    table.insert(lines, string.format("Write:   %d", M.results.errors.write))
    table.insert(lines, string.format("Timeout: %d", M.results.errors.timeout))

    -- Handle errors.status which is a table containing 4xx and 5xx counts
    local status_errors_total = 0
    if type(M.results.errors.status) == "table" then
        for _, count in pairs(M.results.errors.status) do
            if type(count) == "number" then
                status_errors_total = status_errors_total + count
            end
        end
        table.insert(lines, string.format("Status (total): %d", status_errors_total))
        table.insert(lines, string.format("  4xx: %d", M.results.errors.status["4xx"] or 0))
        table.insert(lines, string.format("  5xx: %d", M.results.errors.status["5xx"] or 0))
    elseif type(M.results.errors.status) == "number" then
        table.insert(lines, string.format("Status: %d", M.results.errors.status))
    else
        table.insert(lines, string.format("Status: %s", tostring(M.results.errors.status or 0)))
    end

    -- Also show status_total if available
    if M.results.errors.status_total and M.results.errors.status_total > 0 then
        table.insert(lines, string.format("Status (from wrk): %d", M.results.errors.status_total))
    end

    table.insert(lines, "====================================")

    return table.concat(lines, "\n")
end

-- Print results to stdout
function M.print_results()
    local output
    if M.config.output_format == "json" then
        output = M.format_json()
    elseif M.config.output_format == "yaml" then
        output = M.format_yaml()
    else
        output = M.format_text()
    end

    io.write(output)
    io.write("\n")
end

-- Save results to a file
-- @param filepath string Output file path (optional, uses config if not provided)
function M.save_results(filepath)
    filepath = filepath or M.config.output_file
    if not filepath then
        return
    end

    local output
    if filepath:match("%.json$") then
        output = M.format_json()
    elseif filepath:match("%.ya?ml$") then
        output = M.format_yaml()
    else
        output = M.format_text()
    end

    local file = io.open(filepath, "w")
    if file then
        file:write(output)
        file:write("\n")
        file:close()
        io.write(string.format("[result_collector] Results saved to: %s\n", filepath))
    else
        io.stderr:write(string.format("[result_collector] Failed to save results to: %s\n", filepath))
    end
end

-- Get results as a table (for programmatic access)
-- @return table Results data
function M.get_results()
    return M.results
end

-- Format results in extended JSON format (for profiling integration)
-- This format is compatible with the profile.sh script output
-- @return string Extended JSON formatted results
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

-- Save results in extended JSON format
-- @param filepath string Output file path
function M.save_extended_results(filepath)
    if not filepath then
        return
    end

    local output = M.format_extended_json()
    local file = io.open(filepath, "w")
    if file then
        file:write(output)
        file:write("\n")
        file:close()
        io.write(string.format("[result_collector] Extended results saved to: %s\n", filepath))
    else
        io.stderr:write(string.format("[result_collector] Failed to save extended results to: %s\n", filepath))
    end
end

return M
