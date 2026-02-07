-- Contention Scenario Script for wrk benchmarks
-- benches/api/benchmarks/scripts/contention.lua
--
-- Generates high-contention workloads by targeting a small set of resources.
-- Used for testing Persistent* data structure locking and allocation behavior.
--
-- Environment variables:
--   CONTENTION_LEVEL  - "low", "medium", "high" (default: "medium")
--   WRITE_RATIO       - Write operation ratio 0-100 (default: 50)
--   TARGET_RESOURCES  - Number of resources to target (default: based on contention)
--   BASE_URL          - Base URL for API (default: "http://localhost:3000")
--
-- Concurrency-related environment variables (set by BenchmarkScenario::to_env_vars):
--   WORKER_THREADS    - Number of Axum worker threads
--   DATABASE_POOL_SIZE - Database connection pool size
--   REDIS_POOL_SIZE   - Redis connection pool size
--   MAX_CONNECTIONS   - Maximum simultaneous connections
--
-- Load generation environment variables:
--   CONNECTIONS       - Number of concurrent connections
--   THREADS           - Number of threads
--   DURATION_SECONDS  - Test duration in seconds
--   TARGET_RPS        - Target requests per second (if > 0)
--
-- Usage:
--   wrk -t4 -c100 -d60s -s contention.lua http://localhost:3000
--
-- Using with Rust BenchmarkScenario:
--
--   ```rust
--   use lambars_api_benchmark::infrastructure::{BenchmarkScenario, ConcurrencyConfig, ContentionLevel};
--   use std::process::Command;
--
--   let scenario = BenchmarkScenario::from_file("path/to/scenario.yaml")?;
--   let env_vars = scenario.to_env_vars();
--
--   let mut cmd = Command::new("wrk");
--   cmd.args(["-t4", "-c100", "-d60s", "-s", "contention.lua", "http://localhost:3000"]);
--
--   // Set environment variables from scenario
--   for (key, value) in &env_vars {
--       cmd.env(key, value);
--   }
--
--   let output = cmd.output()?;
--   ```

-- Add scripts directory to package path
local script_dir = debug.getinfo(1, "S").source:match("^@(.+/)")
if script_dir then
    package.path = script_dir .. "?.lua;" .. package.path
end

-- Load common utilities
local common = require("common")
local error_tracker = pcall(require, "error_tracker") and require("error_tracker") or nil

-- Configuration
local config = {
    contention_level = common.getenv("CONTENTION_LEVEL", "medium"),
    write_ratio = common.getenv_number("WRITE_RATIO", 50),
    target_resources = common.getenv_number("TARGET_RESOURCES", 0),
    base_url = common.getenv("BASE_URL", "http://localhost:3000")
}

-- Contention level presets
local contention_presets = {
    low = {
        target_resources = 1000,
        write_ratio = 10
    },
    medium = {
        target_resources = 100,
        write_ratio = 50
    },
    high = {
        target_resources = 10,
        write_ratio = 90
    }
}

-- Apply contention preset
local function apply_preset()
    local preset = contention_presets[config.contention_level]
    if preset then
        if config.target_resources == 0 then
            config.target_resources = preset.target_resources
        end
        -- Use preset write ratio if not explicitly set
        if os.getenv("WRITE_RATIO") == nil then
            config.write_ratio = preset.write_ratio
        end
    else
        -- Default to medium if unknown level
        config.target_resources = config.target_resources > 0 and config.target_resources or 100
    end
end

apply_preset()

-- Pre-generated resource IDs for targeting
local resource_ids = {}
for i = 1, config.target_resources do
    resource_ids[i] = string.format("resource-%03d", i)
end

-- Tracking counters
local counters = {
    reads = 0,
    writes = 0,
    errors = 0,
    conflicts = 0
}
local last_endpoint = nil  -- Track endpoint for per-endpoint metrics
local benchmark_initialized = false

-- Operation types for write operations
local write_operations = {"create", "update", "patch", "delete"}
local read_operations = {"get", "list"}

-- Get a random resource ID (concentrated access for high contention)
local function get_target_resource_id()
    local index = math.random(1, config.target_resources)
    return resource_ids[index]
end

-- Create a task JSON payload
local function create_task_payload(resource_id)
    local task = {
        id = resource_id,
        title = "Contention test task " .. resource_id,
        description = "Testing write contention scenario",
        priority = common.random_priority(),
        status = common.random_status()
    }
    return common.json_encode(task)
end

-- Create an update payload
local function update_payload()
    return common.json_encode({
        title = "Updated: " .. common.random_title(),
        status = common.random_status(),
        priority = common.random_priority()
    })
end

-- Create a status patch payload
local function patch_status_payload()
    return common.json_encode({
        status = common.random_status()
    })
end

-- Decide if this request should be a write operation
local function should_write()
    return math.random(100) <= config.write_ratio
end

-- Generate a request based on operation type
local function generate_request()
    local resource_id = get_target_resource_id()
    local method, path, body, headers
    local endpoint  -- Track endpoint for per-endpoint metrics

    if should_write() then
        -- Write operation
        local op_index = math.random(1, #write_operations)
        local operation = write_operations[op_index]

        if operation == "create" then
            method = "POST"
            path = "/tasks"
            endpoint = "/tasks"
            body = create_task_payload(resource_id)
            headers = {["Content-Type"] = "application/json"}
        elseif operation == "update" then
            method = "PUT"
            path = "/tasks/" .. resource_id
            endpoint = "/tasks/{id}"
            body = update_payload()
            headers = {["Content-Type"] = "application/json"}
        elseif operation == "patch" then
            method = "PATCH"
            path = "/tasks/" .. resource_id .. "/status"
            endpoint = "/tasks/{id}/status"
            body = patch_status_payload()
            headers = {["Content-Type"] = "application/json"}
        elseif operation == "delete" then
            method = "DELETE"
            path = "/tasks/" .. resource_id
            endpoint = "/tasks/{id}"
            body = nil
            headers = {}
        end
        counters.writes = counters.writes + 1
    else
        -- Read operation
        local op_index = math.random(1, #read_operations)
        local operation = read_operations[op_index]

        if operation == "get" then
            method = "GET"
            path = "/tasks/" .. resource_id
            endpoint = "/tasks/{id}"
        else
            method = "GET"
            path = "/tasks?limit=10"
            endpoint = "/tasks"
        end
        body = nil
        headers = {}
        counters.reads = counters.reads + 1
    end

    return method, path, body, headers, endpoint
end

-- wrk setup hook
function setup(thread)
    if not benchmark_initialized then
        common.init_benchmark({scenario_name = "contention", output_format = "json"})
        if error_tracker then error_tracker.init() end
        benchmark_initialized = true
    end
    thread:set("id", thread.id)
    if error_tracker then error_tracker.setup_thread(thread) end
end

-- wrk init hook
function init(args)
    io.write(string.format(
        "[contention] Initialized: level=%s, write_ratio=%d%%, target_resources=%d\n",
        config.contention_level, config.write_ratio, config.target_resources
    ))
end

-- wrk request hook
function request()
    local method, path, body, headers, endpoint = generate_request()
    last_endpoint = endpoint  -- Store endpoint for response() callback

    if body then
        return wrk.format(method, path, headers, body)
    else
        return wrk.format(method, path, headers)
    end
end

-- wrk response hook
function response(status, headers, body)
    if error_tracker then error_tracker.track_thread_response(status) end
    -- Pass headers and endpoint to track_response for cache metrics and per-endpoint tracking
    common.track_response(status, headers, last_endpoint)

    if status >= 400 then
        counters.errors = counters.errors + 1

        -- Track conflict errors (409) specifically
        if status == 409 then
            counters.conflicts = counters.conflicts + 1
        end
    end
end

-- wrk done hook
function done(summary, latency, requests)
    io.write("\n--- Contention Test Summary ---\n")
    io.write(string.format("Contention level: %s\n", config.contention_level))
    io.write(string.format("Target resources: %d\n", config.target_resources))
    io.write(string.format("Write ratio: %d%%\n", config.write_ratio))
    io.write(string.format("\n"))

    -- Use summary for accurate counts (thread-safe)
    local total_requests = summary.requests
    local duration_seconds = summary.duration / 1000000

    io.write(string.format("Total requests: %d\n", total_requests))
    io.write(string.format("Duration: %.1fs\n", duration_seconds))
    io.write(string.format("Actual RPS: %.1f\n", total_requests / duration_seconds))
    io.write(string.format("\n"))

    -- Error summary
    local total_errors = summary.errors.connect + summary.errors.read +
                         summary.errors.write + summary.errors.timeout +
                         summary.errors.status
    local error_rate = total_requests > 0 and (total_errors / total_requests * 100) or 0

    io.write(string.format("Errors: %d (%.2f%%)\n", total_errors, error_rate))
    io.write(string.format("  Connect: %d\n", summary.errors.connect))
    io.write(string.format("  Read: %d\n", summary.errors.read))
    io.write(string.format("  Write: %d\n", summary.errors.write))
    io.write(string.format("  Timeout: %d\n", summary.errors.timeout))
    io.write(string.format("  Status: %d\n", summary.errors.status))
    io.write(string.format("\n"))

    -- Latency statistics
    io.write("Latency:\n")
    io.write(string.format("  Mean: %.2fms\n", latency.mean / 1000))
    io.write(string.format("  Stdev: %.2fms\n", latency.stdev / 1000))
    io.write(string.format("  p50: %.2fms\n", latency:percentile(50) / 1000))
    io.write(string.format("  p90: %.2fms\n", latency:percentile(90) / 1000))
    io.write(string.format("  p99: %.2fms\n", latency:percentile(99) / 1000))
    io.write(string.format("  Max: %.2fms\n", latency.max / 1000))

    -- Print standard summary
    common.print_summary("contention", summary)

    common.finalize_benchmark(summary, latency, requests)
end
