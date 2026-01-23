-- Common utilities for wrk benchmark scripts
-- benches/api/benchmarks/scripts/common.lua
--
-- This module provides:
--   - Basic utilities (UUID, random data generation, JSON encoding)
--   - Integration with load_profile, payload_generator, result_collector modules
--
-- Usage:
--   local common = require("common")
--
--   -- Basic usage (unchanged from original):
--   common.random_uuid()
--   common.json_encode(table)
--   common.track_response(status, headers, endpoint)
--   common.print_summary("script_name", summary)
--
--   -- Extended usage with new modules:
--   common.init_benchmark({
--       scenario_name = "my_benchmark",
--       load_profile = "ramp_up_down",
--       target_rps = 1000,
--       payload_variant = "complex"
--   })
--   -- In done():
--   common.finalize_benchmark(summary, latency, requests)

local M = {}

-- Error tracking
M.status_counts = {
    [200] = 0,
    [201] = 0,
    [400] = 0,
    [404] = 0,
    [422] = 0,
    [500] = 0,
    other = 0
}
M.total_requests = 0

-- Module references (lazy loaded)
M.load_profile = nil
M.payload_generator = nil
M.result_collector = nil

-- Generate a random UUID v4
function M.random_uuid()
    local template = "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx"
    return string.gsub(template, "[xy]", function(c)
        local v = (c == "x") and math.random(0, 0xf) or math.random(8, 0xb)
        return string.format("%x", v)
    end)
end

-- Generate a random task title
function M.random_title()
    local prefixes = {"Implement", "Fix", "Update", "Refactor", "Test", "Deploy", "Review", "Optimize"}
    local subjects = {"authentication", "database", "API", "cache", "logging", "metrics", "UI", "docs"}
    return prefixes[math.random(#prefixes)] .. " " .. subjects[math.random(#subjects)]
end

-- Generate a random priority
function M.random_priority()
    local priorities = {"low", "medium", "high", "critical"}
    return priorities[math.random(#priorities)]
end

-- Generate a random status
function M.random_status()
    local statuses = {"pending", "in_progress", "completed", "cancelled"}
    return statuses[math.random(#statuses)]
end

-- Create an empty array marker
M.EMPTY_ARRAY = setmetatable({}, {__is_array = true})

-- Create an array (ensures proper JSON array encoding)
function M.array(tbl)
    return setmetatable(tbl or {}, {__is_array = true})
end

-- JSON encode a table (simple implementation for benchmark use)
function M.json_encode(tbl)
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

    local mt = getmetatable(tbl)
    local is_array = (mt and mt.__is_array) or #tbl > 0
    local parts = {}

    if is_array then
        for _, v in ipairs(tbl) do
            table.insert(parts, M.json_encode(v))
        end
        return "[" .. table.concat(parts, ",") .. "]"
    else
        for k, v in pairs(tbl) do
            table.insert(parts, '"' .. k .. '":' .. M.json_encode(v))
        end
        return "{" .. table.concat(parts, ",") .. "}"
    end
end

-- Track response status
-- @param status number HTTP status code
-- @param headers table HTTP response headers (optional, for cache detection)
-- @param endpoint string The endpoint path (optional, for per-endpoint metrics)
function M.track_response(status, headers, endpoint)
    M.total_requests = M.total_requests + 1
    if M.status_counts[status] then
        M.status_counts[status] = M.status_counts[status] + 1
    else
        M.status_counts.other = M.status_counts.other + 1
    end

    -- Also track in result_collector if available
    if M.result_collector then
        M.result_collector.record_response(status, nil, headers, endpoint)
    end
end

-- Print status summary (call from done())
-- NOTE: wrk runs multiple threads with separate Lua interpreters.
-- The response() callback runs in worker threads, but done() runs in main thread.
-- Therefore, M.total_requests counted in response() is NOT available in done().
-- Use the summary parameter provided by wrk instead.
function M.print_summary(script_name, summary)
    io.write("\n--- " .. script_name .. " Status Summary ---\n")

    -- Use wrk's summary for accurate counts (thread-safe)
    local total = summary and summary.requests or M.total_requests
    local errors = summary and (summary.errors.connect + summary.errors.read +
                                summary.errors.write + summary.errors.timeout +
                                summary.errors.status) or 0

    io.write(string.format("Total requests: %d\n", total))

    if total > 0 then
        local error_rate = (errors / total) * 100
        io.write(string.format("Errors: %d (%.1f%%)\n", errors, error_rate))
    else
        io.write("Errors: 0 (0.0%)\n")
    end
end

-- =============================================================================
-- Extended benchmark initialization and finalization
-- =============================================================================

-- Try to load a module, return nil if not found
local function try_require(module_name)
    local ok, module = pcall(require, module_name)
    if ok then
        return module
    else
        return nil
    end
end

-- Initialize benchmark with extended options
-- @param options table Configuration options:
--   - scenario_name: Name of the benchmark scenario
--   - load_profile: RPS profile type (constant, ramp_up_down, burst, step_up)
--   - target_rps: Target requests per second
--   - duration_seconds: Benchmark duration
--   - payload_variant: Payload size variant (minimal, standard, complex, heavy)
--   - output_format: Result output format (json, yaml, text)
--   - output_file: Optional file path to save results
--   - Additional load_profile options (ramp_up_seconds, burst_multiplier, etc.)
function M.init_benchmark(options)
    options = options or {}

    -- Load extended modules
    M.load_profile = try_require("load_profile")
    M.payload_generator = try_require("payload_generator")
    M.result_collector = try_require("result_collector")

    -- Initialize load profile
    if M.load_profile and options.load_profile then
        M.load_profile.init({
            profile = options.load_profile,
            target_rps = options.target_rps or 100,
            duration_seconds = options.duration_seconds or 60,
            ramp_up_seconds = options.ramp_up_seconds,
            ramp_down_seconds = options.ramp_down_seconds,
            burst_multiplier = options.burst_multiplier,
            burst_duration_seconds = options.burst_duration_seconds,
            burst_interval_seconds = options.burst_interval_seconds,
            step_count = options.step_count,
            min_rps = options.min_rps
        })
    end

    -- Initialize payload generator
    if M.payload_generator and options.payload_variant then
        M.payload_generator.set_variant(options.payload_variant)
    end

    -- Initialize result collector
    if M.result_collector then
        M.result_collector.init({
            scenario_name = options.scenario_name or "benchmark",
            output_format = options.output_format or "text",
            output_file = options.output_file
        })

        -- Pass metadata from other modules
        if M.load_profile then
            M.result_collector.set_load_profile(M.load_profile.get_profile_metadata())
        end
        if M.payload_generator then
            M.result_collector.set_payload(M.payload_generator.get_metadata())
        end
    end

    io.write(string.format("[common] Benchmark initialized: %s\n", options.scenario_name or "benchmark"))
end

-- Finalize benchmark and print results (call from done())
-- @param summary table wrk summary object (contains thread-aggregated stats)
-- @param latency table wrk latency object
-- @param requests table wrk requests object
-- NOTE: wrk runs multiple threads with separate Lua interpreters.
-- The summary parameter provides accurate, aggregated counts across all threads.
-- Do NOT rely on M.total_requests or M.status_counts in done() - they only
-- contain counts from the main thread (which is typically 0).
function M.finalize_benchmark(summary, latency, requests)
    -- Print load profile summary (pass summary for accurate request counts)
    if M.load_profile then
        M.load_profile.print_profile_summary(summary)
    end

    -- Print payload summary
    if M.payload_generator then
        M.payload_generator.print_summary()
    end

    -- Finalize and print result collector
    if M.result_collector then
        M.result_collector.finalize(summary, latency, requests)
        M.result_collector.print_results()
        M.result_collector.save_results()
    end
end

-- Get current target RPS (delegates to load_profile)
-- @return number Current target RPS, or 0 if load_profile not available
function M.current_target_rps()
    if M.load_profile then
        return M.load_profile.current_target_rps()
    end
    return 0
end

-- Get current load phase name
-- @return string Current phase name
function M.current_phase()
    if M.load_profile then
        return M.load_profile.current_phase()
    end
    return "unknown"
end

-- Generate a task payload using payload_generator
-- @param options table Optional overrides
-- @return string JSON payload
function M.create_task_payload(options)
    if M.payload_generator then
        return M.payload_generator.create_task(options)
    else
        -- Fallback to simple payload
        return M.json_encode({
            title = M.random_title(),
            description = "Benchmark task",
            priority = M.random_priority()
        })
    end
end

-- Generate an update payload using payload_generator
-- @param options table Optional overrides
-- @return string JSON payload
function M.update_task_payload(options)
    if M.payload_generator then
        return M.payload_generator.update_task(options)
    else
        -- Fallback to simple payload
        return M.json_encode({
            title = M.random_title(),
            status = M.random_status()
        })
    end
end

-- =============================================================================
-- Environment variable helpers
-- =============================================================================

-- Get environment variable with default value
-- @param name string Environment variable name
-- @param default any Default value if not set
-- @return any Value or default
function M.getenv(name, default)
    local value = os.getenv(name)
    if value == nil or value == "" then
        return default
    end
    return value
end

-- Get numeric environment variable with default value
-- @param name string Environment variable name
-- @param default number Default value if not set
-- @return number Numeric value or default
function M.getenv_number(name, default)
    local value = os.getenv(name)
    local num = tonumber(value)
    if num == nil then
        return default
    end
    return num
end

-- Initialize random seed (with optional SEED environment variable for reproducibility)
local seed = tonumber(os.getenv("SEED")) or os.time()
math.randomseed(seed)

-- =============================================================================
-- Test IDs helper (shared fallback IDs for benchmark scripts)
-- =============================================================================

-- Default fallback test IDs when test_ids.lua is not available
M.fallback_test_ids = {
    task_ids = {
        "a1b2c3d4-e5f6-4789-abcd-ef0123456789",
        "b2c3d4e5-f6a7-4890-bcde-f01234567890",
        "c3d4e5f6-a7b8-4901-cdef-012345678901"
    },
    project_ids = {
        "f6a7b8c9-d0e1-4234-fabc-345678901234",
        "a7b8c9d0-e1f2-4345-abcd-456789012345"
    }
}

-- Load test IDs with fallback
-- @return table Test IDs module or fallback
function M.load_test_ids()
    local ok, ids = pcall(require, "test_ids")
    if ok then
        return ids
    else
        -- Return fallback with helper functions
        local fallback = M.fallback_test_ids
        fallback.get_task_id = function(index)
            return fallback.task_ids[((index - 1) % #fallback.task_ids) + 1]
        end
        fallback.get_project_id = function(index)
            return fallback.project_ids[((index - 1) % #fallback.project_ids) + 1]
        end
        return fallback
    end
end

-- =============================================================================
-- Standard response/done callbacks for simple benchmark scripts
-- =============================================================================

-- Create a standard response handler for benchmark scripts
-- @param script_name string Name of the script for error logging
-- @return function Response handler function
function M.create_response_handler(script_name)
    return function(status, headers, body)
        M.track_response(status, headers)
        if status >= 400 and status ~= 404 then
            io.stderr:write(string.format("[%s] Error %d\n", script_name, status))
        end
    end
end

-- Create a standard done handler for benchmark scripts
-- @param script_name string Name of the script for summary output
-- @return function Done handler function
function M.create_done_handler(script_name)
    return function(summary, latency, requests)
        M.print_summary(script_name, summary)
    end
end

return M
