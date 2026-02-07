-- Profile benchmark wrk script
-- benches/api/benchmarks/scripts/profile_wrk.lua
--
-- Dedicated wrk script for profile.sh that integrates with result_collector
-- for extended JSON output.
--
-- This script is designed to be used with profile.sh for profiling benchmarks.
-- It provides setup/request/response/done callbacks required by wrk
-- and initializes result_collector to produce proper JSON output.
--
-- IMPORTANT: Thread Isolation
-- ===========================
-- wrk runs each thread with an isolated Lua interpreter. This means:
-- - Global variables are NOT shared between threads
-- - Counters updated in response() are thread-local and NOT aggregated
-- - Use wrk's summary parameter in done() for accurate, thread-aggregated counts
--
-- Usage:
--   wrk -t2 -c10 -d30s -s scripts/profile_wrk.lua http://localhost:3002
--
-- Environment variables:
--   SCENARIO_NAME: Name of the benchmark scenario (default: "profile")
--   LOAD_PROFILE: RPS profile type (default: "constant")
--   TARGET_RPS: Target requests per second (default: 100)
--   PAYLOAD_VARIANT: Payload size variant (default: "standard")
--   OUTPUT_FORMAT: Result output format (default: "json")

package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local error_tracker = pcall(require, "error_tracker") and require("error_tracker") or nil

-- State
local counter = 0
local request_types = {"create", "read", "update", "search"}
local last_endpoint = nil  -- Track endpoint for per-endpoint metrics
local benchmark_initialized = false

local test_ids = common.load_test_ids()

-- Setup function (called once per thread)
function setup(thread)
    if not benchmark_initialized then
        -- Initialize benchmark with profiling settings from environment variables
        common.init_benchmark({
            scenario_name = os.getenv("SCENARIO_NAME") or "profile",
            load_profile = os.getenv("LOAD_PROFILE") or "constant",
            target_rps = tonumber(os.getenv("TARGET_RPS")) or 100,
            payload_variant = os.getenv("PAYLOAD_VARIANT") or "standard",
            output_format = os.getenv("OUTPUT_FORMAT") or "json"  -- Default to JSON for profiling
        })
        if error_tracker then error_tracker.init() end
        benchmark_initialized = true
    end
    if error_tracker then error_tracker.setup_thread(thread) end
end

-- Request generation
function request()
    counter = counter + 1
    local req_type = request_types[(counter % #request_types) + 1]
    local task_id = test_ids.get_task_id(counter)

    if req_type == "create" then
        -- POST /tasks - Create new task
        last_endpoint = "/tasks"
        local body = common.create_task_payload()
        return wrk.format("POST", "/tasks", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "read" then
        -- GET /tasks/{id} - Read existing task
        last_endpoint = "/tasks/{id}"
        return wrk.format("GET", "/tasks/" .. task_id, {})

    elseif req_type == "update" then
        -- PUT /tasks/{id} - Update existing task
        last_endpoint = "/tasks/{id}"
        local body = common.update_task_payload()
        return wrk.format("PUT", "/tasks/" .. task_id, {["Content-Type"] = "application/json"}, body)

    else
        -- POST /tasks/search - Search tasks
        last_endpoint = "/tasks/search"
        local body = common.json_encode({
            query = common.random_title(),
            limit = 20
        })
        return wrk.format("POST", "/tasks/search", {["Content-Type"] = "application/json"}, body)
    end
end

-- Response handler
-- NOTE: This runs in each worker thread. wrk maintains thread-local Lua states,
-- so counters updated here are NOT visible in done() which runs in main thread.
-- Use wrk's summary parameter in done() for accurate, thread-aggregated counts.
function response(status, headers, body)
    if error_tracker then error_tracker.track_thread_response(status) end
    -- Pass headers and endpoint to track_response for cache metrics and per-endpoint tracking
    common.track_response(status, headers, last_endpoint)

    -- Count request for load profile tracking (thread-local)
    if common.load_profile then
        common.load_profile.count_request()
    end
end

-- Done handler (called once at the end)
function done(summary, latency, requests)
    -- Print standard summary first
    common.print_summary("profile", summary)

    -- Finalize and print extended benchmark results
    common.finalize_benchmark(summary, latency, requests)
end
