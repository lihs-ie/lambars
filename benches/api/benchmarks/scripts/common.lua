local M = {}

M.status_counts = {[200] = 0, [201] = 0, [400] = 0, [404] = 0, [422] = 0, [500] = 0, other = 0}
M.total_requests = 0
M.load_profile = nil
M.payload_generator = nil
M.result_collector = nil

function M.random_uuid()
    local template = "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx"
    return string.gsub(template, "[xy]", function(c)
        local v = (c == "x") and math.random(0, 0xf) or math.random(8, 0xb)
        return string.format("%x", v)
    end)
end

local function random_choice(tbl)
    return tbl[math.random(#tbl)]
end

local PREFIXES = {"Implement", "Fix", "Update", "Refactor", "Test", "Deploy", "Review", "Optimize"}
local SUBJECTS = {"authentication", "database", "API", "cache", "logging", "metrics", "UI", "docs"}
local PRIORITIES = {"low", "medium", "high", "critical"}
local STATUSES = {"pending", "in_progress", "completed", "cancelled"}

function M.random_title()
    return random_choice(PREFIXES) .. " " .. random_choice(SUBJECTS)
end

function M.random_priority()
    return random_choice(PRIORITIES)
end

function M.random_status()
    return random_choice(STATUSES)
end

M.EMPTY_ARRAY = setmetatable({}, {__is_array = true})

function M.array(tbl)
    return setmetatable(tbl or {}, {__is_array = true})
end

function M.json_encode(tbl)
    if type(tbl) ~= "table" then
        if type(tbl) == "string" then
            return '"' .. tbl:gsub('\\', '\\\\'):gsub('"', '\\"'):gsub('\n', '\\n') .. '"'
        elseif type(tbl) == "boolean" then return tbl and "true" or "false"
        elseif type(tbl) == "nil" then return "null"
        else return tostring(tbl)
        end
    end

    local mt = getmetatable(tbl)
    local is_array = (mt and mt.__is_array) or #tbl > 0
    local parts = {}

    if is_array then
        for _, v in ipairs(tbl) do table.insert(parts, M.json_encode(v)) end
        return "[" .. table.concat(parts, ",") .. "]"
    else
        for k, v in pairs(tbl) do table.insert(parts, '"' .. k .. '":' .. M.json_encode(v)) end
        return "{" .. table.concat(parts, ",") .. "}"
    end
end

function M.track_response(status, headers, endpoint)
    M.total_requests = M.total_requests + 1
    if M.status_counts[status] then
        M.status_counts[status] = M.status_counts[status] + 1
    else
        M.status_counts.other = M.status_counts.other + 1
    end
    if M.result_collector then
        M.result_collector.record_response(status, nil, headers, endpoint)
    end
end

function M.print_summary(script_name, summary)
    io.write("\n--- " .. script_name .. " Status Summary ---\n")
    local total = summary and summary.requests or M.total_requests
    local errors = summary and (summary.errors.connect + summary.errors.read +
                                summary.errors.write + summary.errors.timeout +
                                summary.errors.status) or 0
    io.write(string.format("Total requests: %d\n", total))
    if total > 0 then
        io.write(string.format("Errors: %d (%.1f%%)\n", errors, (errors / total) * 100))
    else
        io.write("Errors: 0 (0.0%)\n")
    end
end

local function try_require(module_name)
    local ok, module = pcall(require, module_name)
    return ok and module or nil
end

function M.init_benchmark(options)
    options = options or {}
    M.load_profile = try_require("load_profile")
    M.payload_generator = try_require("payload_generator")
    M.result_collector = try_require("result_collector")

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

    if M.payload_generator and options.payload_variant then
        M.payload_generator.set_variant(options.payload_variant)
    end

    if M.result_collector then
        M.result_collector.init({
            scenario_name = options.scenario_name or "benchmark",
            output_format = options.output_format or "text",
            output_file = options.output_file
        })
        if M.load_profile then
            M.result_collector.set_load_profile(M.load_profile.get_profile_metadata())
        end
        if M.payload_generator then
            M.result_collector.set_payload(M.payload_generator.get_metadata())
        end
    end

    io.write(string.format("[common] Benchmark initialized: %s\n", options.scenario_name or "benchmark"))
end

function M.finalize_benchmark(summary, latency, requests)
    if M.load_profile then M.load_profile.print_profile_summary(summary) end
    if M.payload_generator then M.payload_generator.print_summary() end

    if M.result_collector then
        M.result_collector.finalize(summary, latency, requests)
        M.result_collector.print_results()
        local results_dir = os.getenv("LUA_RESULTS_DIR")
        if results_dir and results_dir ~= "" then
            local metrics_file = results_dir .. "/lua_metrics.json"
            M.result_collector.save_results(metrics_file)
            io.write(string.format("[common] Lua metrics saved to: %s\n", metrics_file))
        else
            M.result_collector.save_results()
        end
    end
end

function M.current_target_rps()
    return M.load_profile and M.load_profile.current_target_rps() or 0
end

function M.current_phase()
    return M.load_profile and M.load_profile.current_phase() or "unknown"
end

function M.create_task_payload(options)
    if M.payload_generator then return M.payload_generator.create_task(options) end
    return M.json_encode({
        title = M.random_title(),
        description = "Benchmark task",
        priority = M.random_priority()
    })
end

function M.update_task_payload(options)
    if M.payload_generator then return M.payload_generator.update_task(options) end
    return M.json_encode({title = M.random_title(), status = M.random_status()})
end

function M.getenv(name, default)
    local value = os.getenv(name)
    return (value == nil or value == "") and default or value
end

function M.getenv_number(name, default)
    return tonumber(os.getenv(name)) or default
end

math.randomseed(os.time())

M.fallback_test_ids = {
    task_ids = {"a1b2c3d4-e5f6-4789-abcd-ef0123456789", "b2c3d4e5-f6a7-4890-bcde-f01234567890",
                "c3d4e5f6-a7b8-4901-cdef-012345678901"},
    project_ids = {"f6a7b8c9-d0e1-4234-fabc-345678901234", "a7b8c9d0-e1f2-4345-abcd-456789012345"}
}

local function create_fallback_interface(fallback)
    fallback.task_versions = fallback.task_versions or {}
    for i = 1, #fallback.task_ids do
        fallback.task_versions[i] = fallback.task_versions[i] or 1
    end

    local function normalize_index(index, ids)
        return ((index - 1) % #ids) + 1
    end

    local function copy_table(tbl)
        local copy = {}
        for i, v in ipairs(tbl) do copy[i] = v end
        return copy
    end

    fallback.get_task_id = function(index)
        return fallback.task_ids[normalize_index(index, fallback.task_ids)], nil
    end
    fallback.get_project_id = function(index)
        return fallback.project_ids[normalize_index(index, fallback.project_ids)], nil
    end
    fallback.get_task_count = function() return #fallback.task_ids end
    fallback.get_project_count = function() return #fallback.project_ids end
    fallback.get_task_state = function(index)
        local idx = normalize_index(index, fallback.task_ids)
        return {id = fallback.task_ids[idx], version = fallback.task_versions[idx], status = "pending"}, nil
    end
    fallback.set_version = function(index, version)
        fallback.task_versions[normalize_index(index, fallback.task_ids)] = version
        return true, nil
    end
    fallback.increment_version = function(index)
        local idx = normalize_index(index, fallback.task_ids)
        fallback.task_versions[idx] = fallback.task_versions[idx] + 1
        return fallback.task_versions[idx], nil
    end
    fallback.reset_versions = function()
        for i = 1, #fallback.task_ids do fallback.task_versions[i] = 1 end
    end
    fallback.get_all_task_ids = function() return copy_table(fallback.task_ids) end
    fallback.get_all_project_ids = function() return copy_table(fallback.project_ids) end
    return fallback
end

function M.load_test_ids()
    local ok, ids = pcall(require, "test_ids")
    if ok then return ids end
    return create_fallback_interface(M.fallback_test_ids)
end

function M.create_response_handler(script_name)
    return function(status, headers, body)
        M.track_response(status, headers)
        if status >= 400 and status ~= 404 then
            io.stderr:write(string.format("[%s] Error %d\n", script_name, status))
        end
    end
end

function M.create_done_handler(script_name)
    return function(summary, latency, requests)
        M.print_summary(script_name, summary)
        M.finalize_benchmark(summary, latency, requests)
    end
end

function M.track_retry()
    if M.result_collector then M.result_collector.track_retry() end
end

function M.extract_version(body)
    if type(body) ~= "string" or body == "" then return nil end
    local version_str = body:match('"version"%s*:%s*(%d+)') or body:match('"version"%s*:%s*"(%d+)"')
    if not version_str then return nil end
    local version = tonumber(version_str)
    if not version or version < 1 or version ~= math.floor(version) then return nil end
    return version
end

function M.extract_status(body)
    if type(body) ~= "string" or body == "" then return nil end
    return body:match('"status"%s*:%s*"([^"]+)"')
end

local STATUS_LABELS = {
    {code = "200 OK", key = "status_200"},
    {code = "201 Created", key = "status_201"},
    {code = "207 Multi-Status", key = "status_207"},
    {code = "400 Bad Request", key = "status_400"},
    {code = "404 Not Found", key = "status_404"},
    {code = "409 Conflict", key = "status_409"},
    {code = "422 Unprocessable Entity", key = "status_422"},
    {code = "500 Internal Server Error", key = "status_500"},
    {code = "502 Bad Gateway", key = "status_502"},
    {code = "Other Status", key = "status_other"}
}

local function print_status_distribution(script_name, aggregated, total)
    io.write(string.format("\n--- %s HTTP Status Distribution ---\n", script_name))
    if total <= 0 then
        io.write("  No requests completed\n")
        return
    end

    for _, label in ipairs(STATUS_LABELS) do
        local count = aggregated[label.key]
        if count > 0 then
            io.write(string.format("  %s: %d (%.1f%%)\n", label.code, count, (count / total) * 100))
        end
    end

    local errors = aggregated.status_400 + aggregated.status_404 + aggregated.status_409 +
                   aggregated.status_422 + aggregated.status_500 + aggregated.status_502 +
                   aggregated.status_other
    io.write(string.format("Error Rate: %.2f%% (%d errors / %d requests)\n",
        (errors / total) * 100, errors, total))
end

function M.create_threaded_handlers(script_name)
    local error_tracker = try_require("error_tracker")
    if not error_tracker then
        io.stderr:write(string.format("[%s] Warning: error_tracker module not found\n", script_name))
        return {
            setup = function(thread) end,
            response = M.create_response_handler(script_name),
            done = M.create_done_handler(script_name)
        }
    end

    return {
        setup = function(thread) error_tracker.setup_thread(thread) end,
        response = function(status, headers, body)
            error_tracker.track_thread_response(status)
            M.track_response(status, headers)
            if status >= 400 and status ~= 404 then
                io.stderr:write(string.format("[%s] Error %d\n", script_name, status))
            end
        end,
        done = function(summary, latency, requests)
            M.print_summary(script_name, summary)
            local aggregated = error_tracker.get_all_threads_aggregated_summary()
            print_status_distribution(script_name, aggregated, summary.requests or 0)
        end
    }
end

function M.create_standard_handlers(script_name, init_opts)
    local error_tracker = try_require("error_tracker")
    local initialized = false

    local function ensure_initialized()
        if initialized then return end
        initialized = true
        M.init_benchmark(init_opts or {scenario_name = script_name, output_format = "json"})
        if error_tracker then error_tracker.init() end
    end

    return {
        init = function(args)
            ensure_initialized()
        end,
        setup = function(thread)
            ensure_initialized()
            if error_tracker then error_tracker.setup_thread(thread) end
        end,
        response = function(status, headers, body)
            if error_tracker then error_tracker.track_thread_response(status) end
            M.track_response(status, headers)
            if status >= 400 and status ~= 404 then
                io.stderr:write(string.format("[%s] Error %d\n", script_name, status))
            end
        end,
        done = function(summary, latency, requests)
            M.print_summary(script_name, summary)
            if error_tracker then
                print_status_distribution(script_name, error_tracker.get_all_threads_aggregated_summary(),
                                         summary.requests or 0)
            end
            M.finalize_benchmark(summary, latency, requests)
        end
    }
end

return M
