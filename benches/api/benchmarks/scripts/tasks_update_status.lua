package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local test_ids = common.load_test_ids()

local RETRY_COUNT = tonumber(os.getenv("RETRY_COUNT")) or 1
local BACKOFF_BASE = 2
local BACKOFF_MAX = tonumber(os.getenv("RETRY_BACKOFF_MAX")) or 16

local VALID_TRANSITIONS = {
    ["pending"] = {"in_progress", "cancelled"},
    ["in_progress"] = {"completed", "pending", "cancelled"},
    ["completed"] = {"pending"},
    ["cancelled"] = {}
}

local state = "update"
local retry_index, retry_body, retry_attempt = nil, nil, 0
local counter = 0
local last_request_index, last_request_is_update = nil, false
local last_request_status = nil
local retry_sent_status = nil
local thread_retry_count, thread_retry_exhausted_count = 0, 0

local backoff_skip_counter, backoff_skip_target = 0, 0
local is_backoff_request, is_suppressed_request, is_fallback_request = false, false, false

local request_categories = {
    executed = 0,
    backoff = 0,
    suppressed = 0,
    fallback = 0
}

local error_tracker = pcall(require, "error_tracker") and require("error_tracker") or nil
local benchmark_initialized = false

function init(args)
    if error_tracker then error_tracker.init() end
    common.init_benchmark({scenario_name = "tasks_update_status", output_format = "json"})
end

local function validate_and_clamp(value, min_val, default_val, name)
    if not value or value < min_val then
        io.stderr:write(string.format("[tasks_update_status] WARN: Invalid %s, defaulting to %d\n", name, default_val))
        return default_val
    end
    return value
end

function setup(thread)
    if not benchmark_initialized then
        common.init_benchmark({scenario_name = "tasks_update_status", output_format = "json"})
        benchmark_initialized = true
    end

    if error_tracker then error_tracker.setup_thread(thread) end

    local total_threads = validate_and_clamp(tonumber(os.getenv("THREADS") or os.getenv("WRK_THREADS")), 1, 1, "THREADS")
    local pool_size = validate_and_clamp(tonumber(os.getenv("ID_POOL_SIZE")), 1, 10, "ID_POOL_SIZE")
    local thread_id = tonumber(thread.id) or 0

    if pool_size < total_threads then
        io.stderr:write(string.format(
            "[tasks_update_status] WARN: ID_POOL_SIZE (%d) < THREADS (%d), using pool_size=%d\n",
            pool_size, total_threads, pool_size))
        total_threads = pool_size
    end

    if thread_id >= pool_size then
        io.stderr:write(string.format("[tasks_update_status] Thread %d suppressed (ID_POOL_SIZE=%d)\n", thread_id, pool_size))
        thread:set("suppressed", true)
        thread:set("id", thread_id)
        thread:set("id_start", 0)
        thread:set("id_end", 0)
        thread:set("id_range", 0)
        return
    end

    local ids_per_thread = math.floor(pool_size / total_threads)
    local start_index = thread_id * ids_per_thread

    thread:set("suppressed", false)
    thread:set("id", thread_id)
    thread:set("id_start", start_index)
    thread:set("id_end", start_index + ids_per_thread - 1)
    thread:set("id_range", ids_per_thread)

    io.write(string.format("[Thread %d] ID range: %d-%d (%d IDs)\n",
        thread_id, start_index, start_index + ids_per_thread - 1, ids_per_thread))
end

local function next_valid_status(current_status)
    local candidates = VALID_TRANSITIONS[current_status]
    if not candidates or #candidates == 0 then
        return nil
    end
    return candidates[math.random(#candidates)]
end

local function generate_update_body(current_status, version, task_index)
    local next_status = next_valid_status(current_status)
    if not next_status then
        -- Terminal state (e.g., cancelled) reached - skip this task
        return nil
    end
    return common.json_encode({ status = next_status, version = version })
end

local function reset_retry_state()
    state = "update"
    retry_index, retry_body, retry_attempt = nil, nil, 0
    backoff_skip_target = 0
end

local function apply_backoff()
    backoff_skip_target = math.random(0, math.min(BACKOFF_BASE ^ retry_attempt, BACKOFF_MAX))
    backoff_skip_counter = 0
end

local function health_request()
    return wrk and wrk.format and wrk.format("GET", "/health") or ""
end

local function fallback_request()
    reset_retry_state()
    state = "fallback"
    last_request_is_update = false
    is_fallback_request = true
    request_categories.fallback = request_categories.fallback + 1
    return health_request()
end

function request()
    if wrk and wrk.thread then
        local suppressed = wrk.thread:get("suppressed")
        if suppressed == true or suppressed == "true" then
            is_suppressed_request = true
            request_categories.suppressed = request_categories.suppressed + 1
            return health_request()
        end
    end
    is_suppressed_request = false

    if backoff_skip_counter < backoff_skip_target then
        backoff_skip_counter = backoff_skip_counter + 1
        is_backoff_request = true
        request_categories.backoff = request_categories.backoff + 1
        return health_request()
    end
    backoff_skip_counter = 0
    is_backoff_request = false

    if state == "retry_get" then
        local task_id, err = test_ids.get_task_id(retry_index)
        if err then
            io.stderr:write("[tasks_update_status] Error getting task ID for retry: " .. err .. "\n")
            return fallback_request()
        end
        last_request_is_update = true
        return wrk and wrk.format and wrk.format("GET", "/tasks/" .. task_id, {["Accept"] = "application/json"}) or ""
    elseif state == "retry_patch" then
        local task_id, err = test_ids.get_task_id(retry_index)
        if err or not retry_body then
            io.stderr:write("[tasks_update_status] Error in retry_patch: " .. (err or "retry_body is nil") .. "\n")
            return fallback_request()
        end
        last_request_is_update = true
        return wrk and wrk.format and wrk.format("PATCH", "/tasks/" .. task_id .. "/status", {["Content-Type"] = "application/json"}, retry_body) or ""
    elseif state == "fallback" then
        state = "update"
    end

    local id_start = 0
    local id_range = test_ids.get_task_count()

    if wrk and wrk.thread then
        id_start = tonumber(wrk.thread:get("id_start")) or 0
        id_range = tonumber(wrk.thread:get("id_range")) or test_ids.get_task_count()
    end

    -- Find next non-terminal task (skip cancelled tasks)
    local max_attempts = id_range
    local attempt = 0
    local task_state, global_index, body

    repeat
        counter = counter + 1
        global_index = id_start + (counter % id_range) + 1
        attempt = attempt + 1

        local err
        task_state, err = test_ids.get_task_state(global_index)
        if err then
            io.stderr:write("[tasks_update_status] Error getting task state: " .. err .. "\n")
            return fallback_request()
        end

        body = generate_update_body(task_state.status, task_state.version, global_index)
    until body or attempt >= max_attempts

    if not body then
        io.stderr:write("[tasks_update_status] All tasks in terminal state, using fallback\n")
        return fallback_request()
    end

    last_request_index = global_index
    last_request_is_update = true
    last_request_status = body:match('"status"%s*:%s*"([^"]+)"')

    return wrk and wrk.format and wrk.format("PATCH", "/tasks/" .. task_state.id .. "/status", {["Content-Type"] = "application/json"}, body) or ""
end

function response(status, headers, body)
    if not status then return end

    common.track_response(status, headers)
    if error_tracker then error_tracker.track_thread_response(status) end

    if is_backoff_request then
        is_backoff_request = false
        return
    end
    if is_suppressed_request then
        is_suppressed_request = false
        return
    end
    if is_fallback_request then
        is_fallback_request = false
        return
    end

    request_categories.executed = request_categories.executed + 1
    if not last_request_is_update then return end

    if state == "retry_get" then
        if status == 200 then
            local version = common.extract_version(body)
            local retry_status = common.extract_status(body)
            if version and retry_status and common.is_valid_status(retry_status) then
                local success, err = test_ids.set_version_and_status(retry_index, version, retry_status)
                if not success then
                    io.stderr:write("[tasks_update_status] Failed to set version and status: " .. (err or "unknown") .. "\n")
                    reset_retry_state()
                    return
                end
                retry_body = generate_update_body(retry_status, version, retry_index)
                if not retry_body then
                    reset_retry_state()
                    return
                end
                retry_sent_status = retry_body:match('"status"%s*:%s*"([^"]+)"')
                state = "retry_patch"
            else
                io.stderr:write("[tasks_update_status] Failed to extract version or status from GET response\n")
                reset_retry_state()
            end
        else
            io.stderr:write(string.format("[tasks_update_status] Retry GET failed with status %d\n", status))
            reset_retry_state()
        end
    elseif state == "retry_patch" then
        if status == 200 or status == 201 then
            local new_version, err = test_ids.increment_version(retry_index)
            if err then
                io.stderr:write("[tasks_update_status] Error incrementing version after retry: " .. err .. "\n")
            elseif retry_sent_status then
                local success, set_err = test_ids.set_version_and_status(retry_index, new_version, retry_sent_status)
                if not success then
                    io.stderr:write("[tasks_update_status] Error setting status after retry: " .. (set_err or "unknown") .. "\n")
                end
            end
            thread_retry_count = thread_retry_count + 1
            reset_retry_state()
        elseif status == 409 then
            retry_attempt = retry_attempt + 1
            if retry_attempt >= RETRY_COUNT then
                io.stderr:write(string.format("[tasks_update_status] Retry exhausted after %d attempts\n", RETRY_COUNT))
                thread_retry_exhausted_count = thread_retry_exhausted_count + 1
                reset_retry_state()
            else
                io.stderr:write(string.format("[tasks_update_status] Retry PATCH got 409 (attempt %d/%d)\n", retry_attempt, RETRY_COUNT))
                apply_backoff()
                state = "retry_get"
            end
        else
            io.stderr:write(string.format("[tasks_update_status] Retry PATCH failed with status %d\n", status))
            reset_retry_state()
        end
    elseif state == "update" then
        if status == 200 or status == 201 then
            if last_request_index and last_request_status then
                local new_version, err = test_ids.increment_version(last_request_index)
                if err then
                    io.stderr:write("[tasks_update_status] Error incrementing version: " .. err .. "\n")
                else
                    local success, set_err = test_ids.set_version_and_status(last_request_index, new_version, last_request_status)
                    if not success then
                        io.stderr:write("[tasks_update_status] Error setting status: " .. (set_err or "unknown") .. "\n")
                    end
                end
            end
        elseif status == 409 then
            if last_request_index then
                retry_index = last_request_index
                retry_attempt = 0
                if RETRY_COUNT == 0 then
                    io.stderr:write("[tasks_update_status] Conflict detected but retries disabled\n")
                    thread_retry_exhausted_count = thread_retry_exhausted_count + 1
                    reset_retry_state()
                else
                    apply_backoff()
                    state = "retry_get"
                end
            end
        elseif status >= 400 then
            io.stderr:write(string.format("[tasks_update_status] Error %d\n", status))
        end
    end
end

local STATUS_LABELS = {
    {"200 OK", "status_200"}, {"201 Created", "status_201"}, {"207 Multi-Status", "status_207"},
    {"400 Bad Request", "status_400"}, {"404 Not Found", "status_404"}, {"409 Conflict", "status_409"},
    {"422 Unprocessable Entity", "status_422"}, {"500 Internal Server Error", "status_500"},
    {"502 Bad Gateway", "status_502"}, {"Other Status", "status_other"}
}

local function print_excluded_requests()
    local excluded = {{"Backoff", request_categories.backoff},
                      {"Suppressed thread", request_categories.suppressed},
                      {"Fallback", request_categories.fallback}}
    for _, item in ipairs(excluded) do
        if item[2] > 0 then
            io.stderr:write(string.format("[tasks_update_status] %s requests (excluded): %d\n", item[1], item[2]))
        end
    end
end

local function print_status_distribution(aggregated, lua_total, summary)
    io.write("\n--- tasks_update_status HTTP Status Distribution (all threads) ---\n")

    local total = 0
    for _, label in ipairs(STATUS_LABELS) do
        total = total + (aggregated[label[2]] or 0)
    end

    if total <= 0 then
        io.write("  No requests completed\n")
        return
    end

    local error_count = 0
    for _, label in ipairs(STATUS_LABELS) do
        local code, key = label[1], label[2]
        local count = aggregated[key] or 0
        if count > 0 then
            io.write(string.format("  %s: %d (%.1f%%)\n", code, count, (count / total) * 100))
            local status_num = tonumber(key:match("status_(%d+)"))
            if (status_num and status_num >= 400 and status_num < 600) or key == "status_other" then
                error_count = error_count + count
            end
        end
    end

    local actual_error_rate = (error_count / total) * 100
    io.write(string.format("\nActual Error Rate: %.2f%% (%d errors / %d requests)\n",
        actual_error_rate, error_count, total))
    io.write(string.format("Note: %d backoff + %d suppressed + %d fallback requests are excluded from metrics\n",
        request_categories.backoff, request_categories.suppressed, request_categories.fallback))
end

local function aggregate_categories(categories)
    return {
        executed = categories.executed,
        backoff = categories.backoff,
        suppressed = categories.suppressed,
        fallback = categories.fallback
    }
end

local function verify_consistency(categories, total_requests)
    local sum = categories.executed + categories.backoff +
                categories.suppressed + categories.fallback
    return sum == total_requests, sum
end

function done(summary, latency, requests)
    local categories = aggregate_categories(request_categories)
    local is_consistent, sum = verify_consistency(categories, summary.requests)

    if not is_consistent then
        io.stderr:write(string.format(
            "[tasks_update_status] WARN: Inconsistency detected: total=%d, sum(categories)=%d\n",
            summary.requests, sum))
    end

    io.write("\n--- Request Categories ---\n")
    io.write(string.format("  Executed:   %d\n", categories.executed))
    io.write(string.format("  Backoff:    %d\n", categories.backoff))
    io.write(string.format("  Suppressed: %d\n", categories.suppressed))
    io.write(string.format("  Fallback:   %d\n", categories.fallback))
    io.write(string.format("  Total:      %d\n", sum))
    io.write(string.format("  Excluded:   %d (backoff + suppressed + fallback)\n",
        categories.backoff + categories.suppressed + categories.fallback))
    io.write("\n")

    print_excluded_requests()
    common.print_summary("tasks_update_status", summary)

    if error_tracker then
        print_status_distribution(error_tracker.get_all_threads_aggregated_summary(), common.total_requests, summary)
    end

    io.stderr:write(string.format("[tasks_update_status] Retry config: RETRY_COUNT=%d, BACKOFF_MAX=%d\n", RETRY_COUNT, BACKOFF_MAX))

    if thread_retry_count > 0 then
        io.stderr:write(string.format("[tasks_update_status] Thread successful retries: %d\n", thread_retry_count))
    end
    if thread_retry_exhausted_count > 0 then
        io.stderr:write(string.format("[tasks_update_status] Thread retry exhausted: %d\n", thread_retry_exhausted_count))
    end

    local rc = pcall(require, "result_collector") and require("result_collector") or nil
    if rc and rc.set_request_categories then
        rc.set_request_categories(categories)
    end

    common.finalize_benchmark(summary, latency, requests)
end
