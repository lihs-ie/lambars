-- Load Shape Demo Script
-- benches/api/benchmarks/scripts/load_shape_demo.lua
--
-- Demonstrates the usage of load_profile, payload_generator, and result_collector
-- modules with various RPS profiles and payload variants.
--
-- IMPORTANT: Thread Isolation and Rate Limiting
-- =============================================
-- wrk runs each thread with an isolated Lua interpreter. This means:
-- - Global variables are NOT shared between threads
-- - Counters updated in response() are thread-local and NOT aggregated
-- - Use wrk's summary parameter in done() for accurate, thread-aggregated counts
--
-- To achieve proper rate limiting, you MUST use wrk2 with the -R (--rate) option.
-- Standard wrk does NOT support rate limiting; it generates as many requests as possible.
-- Without rate limiting, the load profile (ramp_up_down, burst, etc.) will not be
-- applied as intended because each thread runs independently.
--
-- Usage (wrk2 with rate limiting - RECOMMENDED):
--   # Ramp up/down with standard payload
--   wrk2 -t4 -c30 -d120s -R500 -s scripts/load_shape_demo.lua \
--       --latency http://localhost:8080 \
--       -- --profile=ramp_up_down --payload=standard --target-rps=500
--        ^-- wrk2 with -R (rate) option for proper RPS control
--
--   # Burst test with complex payload
--   wrk2 -t8 -c50 -d120s -R1000 -s scripts/load_shape_demo.lua \
--       --latency http://localhost:8080 \
--       -- --profile=burst --payload=complex --target-rps=1000
--
--   # Step-up stress test with minimal payload
--   wrk2 -t8 -c40 -d180s -R1000 -s scripts/load_shape_demo.lua \
--       --latency http://localhost:8080 \
--       -- --profile=step_up --payload=minimal --target-rps=1000
--
-- Usage (standard wrk - no rate limiting):
--   # Standard wrk (generates max RPS, load profile not enforced)
--   wrk -t4 -c30 -d120s -s scripts/load_shape_demo.lua \
--       --latency http://localhost:8080 \
--       -- --profile=constant --payload=standard
--
-- Environment variables (alternative to command line args):
--   LOAD_PROFILE: ramp_up_down, burst, step_up, constant
--   PAYLOAD_VARIANT: minimal, standard, complex, heavy
--   TARGET_RPS: Target requests per second
--   OUTPUT_FORMAT: json, yaml, text

package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local error_tracker = pcall(require, "error_tracker") and require("error_tracker") or nil

-- Parse command line arguments (after --)
local function parse_args()
    local args = {
        profile = "constant",
        payload = "standard",
        target_rps = 100,
        output_format = "text"
    }

    -- Check environment variables first
    args.profile = os.getenv("LOAD_PROFILE") or args.profile
    args.payload = os.getenv("PAYLOAD_VARIANT") or args.payload
    args.target_rps = tonumber(os.getenv("TARGET_RPS")) or args.target_rps
    args.output_format = os.getenv("OUTPUT_FORMAT") or args.output_format

    -- Parse command line arguments (override env vars)
    for i, arg in ipairs(wrk.args or {}) do
        local key, value = arg:match("^%-%-([%w%-]+)=(.+)$")
        if key == "profile" then
            args.profile = value
        elseif key == "payload" then
            args.payload = value
        elseif key == "target-rps" then
            args.target_rps = tonumber(value) or args.target_rps
        elseif key == "output-format" then
            args.output_format = value
        end
    end

    return args
end

local test_ids = common.load_test_ids()

-- State
local args = nil
local counter = 0
local request_types = {"create", "read", "update", "search"}
local last_endpoint = nil  -- Track endpoint for per-endpoint metrics
local benchmark_initialized = false

-- Setup function (called once per thread)
function setup(thread)
    if not benchmark_initialized then
        args = parse_args()

        -- Initialize benchmark with parsed arguments
        common.init_benchmark({
            scenario_name = "load_shape_demo",
            load_profile = args.profile,
            target_rps = args.target_rps,
            payload_variant = args.payload,
            output_format = args.output_format,
            -- Profile-specific options with sensible defaults
            ramp_up_seconds = 20,
            ramp_down_seconds = 20,
            burst_multiplier = 3.0,
            burst_duration_seconds = 5,
            burst_interval_seconds = 20,
            step_count = 4,
            min_rps = 10
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

    if status >= 400 and status ~= 404 then
        io.stderr:write(string.format("[load_shape_demo] Error %d: %s\n", status, body:sub(1, 100)))
    end
end

-- Done handler (called once at the end)
function done(summary, latency, requests)
    -- Print standard summary first
    common.print_summary("load_shape_demo", summary)

    -- Finalize and print extended benchmark results
    common.finalize_benchmark(summary, latency, requests)
end
