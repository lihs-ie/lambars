package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local test_ids = common.load_test_ids()

local RETRY_COUNT = tonumber(os.getenv("RETRY_COUNT")) or 0
local BACKOFF_BASE = 2
local BACKOFF_MAX = tonumber(os.getenv("RETRY_BACKOFF_MAX")) or 16

local state = "update"
local retry_index, retry_body, retry_attempt = nil, nil, 0
local counter = 0
local last_request_index, last_request_is_update = nil, false
local thread_retry_count, thread_retry_exhausted_count = 0, 0
local update_types = {"priority", "status", "description", "title", "full"}

local backoff_skip_counter, backoff_skip_target = 0, 0
local is_backoff_request, is_suppressed_request, is_fallback_request = false, false, false

-- REQ-MEASURE-401: リクエストカテゴリの追加
request_categories = {
    executed = 0,
    backoff = 0,
    suppressed = 0,
    fallback = 0
}

local error_tracker = pcall(require, "error_tracker") and require("error_tracker") or nil
local benchmark_initialized = false

function init(args)
    if error_tracker then error_tracker.init() end
    common.init_benchmark({scenario_name = "tasks_update", output_format = "json"})
end

local function validate_and_clamp(value, min_val, default_val, name)
    if not value or value < min_val then
        io.stderr:write(string.format("[tasks_update] WARN: Invalid %s, defaulting to %d\n", name, default_val))
        return default_val
    end
    return value
end

function setup(thread)
    if not benchmark_initialized then
        common.init_benchmark({scenario_name = "tasks_update", output_format = "json"})
        benchmark_initialized = true
    end

    if error_tracker then error_tracker.setup_thread(thread) end

    local total_threads = validate_and_clamp(tonumber(os.getenv("WRK_THREADS")), 1, 1, "WRK_THREADS")
    local pool_size = validate_and_clamp(tonumber(os.getenv("ID_POOL_SIZE")), 1, 10, "ID_POOL_SIZE")
    local thread_id = tonumber(thread.id) or 0

    if pool_size < total_threads then
        io.stderr:write(string.format(
            "[tasks_update] WARN: ID_POOL_SIZE (%d) < WRK_THREADS (%d), using pool_size=%d\n",
            pool_size, total_threads, pool_size))
        total_threads = pool_size
    end

    if thread_id >= pool_size then
        io.stderr:write(string.format("[tasks_update] Thread %d suppressed (ID_POOL_SIZE=%d)\n", thread_id, pool_size))
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

local function generate_update_body(update_type, version, request_counter)
    local timestamp_str = tostring(request_counter)
    if update_type == "priority" then
        return common.json_encode({ priority = common.random_priority(), version = version })
    elseif update_type == "status" then
        return common.json_encode({ status = common.random_status(), version = version })
    elseif update_type == "description" then
        return common.json_encode({
            description = "Updated description via Optional optic - request " .. timestamp_str,
            version = version
        })
    elseif update_type == "title" then
        return common.json_encode({ title = common.random_title() .. " (updated)", version = version })
    else
        return common.json_encode({
            title = common.random_title() .. " (full update)",
            description = "Full update via combined optics",
            priority = common.random_priority(),
            status = common.random_status(),
            tags = common.array({"updated", "benchmark"}),
            version = version
        })
    end
end

local function reset_retry_state()
    state = "update"
    retry_index, retry_body, retry_attempt = nil, nil, 0
    backoff_skip_target = 0
end

local function apply_backoff()
    backoff_skip_target = math.min(BACKOFF_BASE ^ retry_attempt, BACKOFF_MAX)
    backoff_skip_counter = 0
end

local function create_health_request(request_type, counter_field)
    _G[counter_field] = _G[counter_field] + 1
    return wrk and wrk.format and wrk.format("GET", "/health") or ""
end

local function fallback_request()
    reset_retry_state()
    state = "fallback"
    last_request_is_update = false
    is_fallback_request = true
    request_categories.fallback = request_categories.fallback + 1
    return wrk and wrk.format and wrk.format("GET", "/health") or ""
end

function request()
    if wrk and wrk.thread then
        local suppressed = wrk.thread:get("suppressed")
        if suppressed == true or suppressed == "true" then
            is_suppressed_request = true
            request_categories.suppressed = request_categories.suppressed + 1
            return wrk and wrk.format and wrk.format("GET", "/health") or ""
        end
    end
    is_suppressed_request = false

    if backoff_skip_counter < backoff_skip_target then
        backoff_skip_counter = backoff_skip_counter + 1
        is_backoff_request = true
        request_categories.backoff = request_categories.backoff + 1
        return wrk and wrk.format and wrk.format("GET", "/health") or ""
    end
    backoff_skip_counter = 0
    is_backoff_request = false

    if state == "retry_get" then
        local task_id, err = test_ids.get_task_id(retry_index)
        if err then
            io.stderr:write("[tasks_update] Error getting task ID for retry: " .. err .. "\n")
            return fallback_request()
        end
        last_request_is_update = true
        return wrk and wrk.format and wrk.format("GET", "/tasks/" .. task_id, {["Accept"] = "application/json"}) or ""
    elseif state == "retry_put" then
        local task_id, err = test_ids.get_task_id(retry_index)
        if err or not retry_body then
            io.stderr:write("[tasks_update] Error in retry_put: " .. (err or "retry_body is nil") .. "\n")
            return fallback_request()
        end
        last_request_is_update = true
        return wrk and wrk.format and wrk.format("PUT", "/tasks/" .. task_id, {["Content-Type"] = "application/json"}, retry_body) or ""
    elseif state == "fallback" then
        state = "update"
    end

    local id_start = 0
    local id_range = test_ids.get_task_count()

    if wrk and wrk.thread then
        id_start = tonumber(wrk.thread:get("id_start")) or 0
        id_range = tonumber(wrk.thread:get("id_range")) or test_ids.get_task_count()
    end

    counter = counter + 1
    local global_index = id_start + (counter % id_range) + 1

    local task_state, err = test_ids.get_task_state(global_index)
    if err then
        io.stderr:write("[tasks_update] Error getting task state: " .. err .. "\n")
        return fallback_request()
    end

    last_request_index = global_index
    last_request_is_update = true

    local update_type = update_types[(counter % #update_types) + 1]
    local body = generate_update_body(update_type, task_state.version, counter)
    return wrk and wrk.format and wrk.format("PUT", "/tasks/" .. task_state.id, {["Content-Type"] = "application/json"}, body) or ""
end

function response(status, headers, body)
    if not status then return end
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

    common.track_response(status, headers)
    request_categories.executed = request_categories.executed + 1
    if error_tracker then error_tracker.track_thread_response(status) end
    if not last_request_is_update then return end

    if state == "retry_get" then
        if status == 200 then
            local version = common.extract_version(body)
            if version then
                local success, err = test_ids.set_version(retry_index, version)
                if not success then
                    io.stderr:write("[tasks_update] Failed to set version: " .. (err or "unknown") .. "\n")
                    reset_retry_state()
                    return
                end
                retry_body = generate_update_body(update_types[(retry_index % #update_types) + 1], version, retry_index)
                state = "retry_put"
            else
                io.stderr:write("[tasks_update] Failed to extract version from GET response\n")
                reset_retry_state()
            end
        else
            io.stderr:write(string.format("[tasks_update] Retry GET failed with status %d\n", status))
            reset_retry_state()
        end
    elseif state == "retry_put" then
        if status == 200 or status == 201 then
            local new_version, err = test_ids.increment_version(retry_index)
            if err then io.stderr:write("[tasks_update] Error incrementing version after retry: " .. err .. "\n") end
            thread_retry_count = thread_retry_count + 1
            reset_retry_state()
        elseif status == 409 then
            retry_attempt = retry_attempt + 1
            if retry_attempt >= RETRY_COUNT then
                io.stderr:write(string.format("[tasks_update] Retry exhausted after %d attempts\n", RETRY_COUNT))
                thread_retry_exhausted_count = thread_retry_exhausted_count + 1
                reset_retry_state()
            else
                io.stderr:write(string.format("[tasks_update] Retry PUT got 409 (attempt %d/%d)\n", retry_attempt, RETRY_COUNT))
                apply_backoff()
                state = "retry_get"
            end
        else
            io.stderr:write(string.format("[tasks_update] Retry PUT failed with status %d\n", status))
            reset_retry_state()
        end
    elseif state == "update" then
        if status == 200 or status == 201 then
            if last_request_index then
                local new_version, err = test_ids.increment_version(last_request_index)
                if err then io.stderr:write("[tasks_update] Error incrementing version: " .. err .. "\n") end
            end
        elseif status == 409 then
            if last_request_index then
                retry_index = last_request_index
                retry_attempt = 0
                if RETRY_COUNT == 0 then
                    io.stderr:write("[tasks_update] Conflict detected but retries disabled\n")
                    thread_retry_exhausted_count = thread_retry_exhausted_count + 1
                    reset_retry_state()
                else
                    apply_backoff()
                    state = "retry_get"
                end
            end
        elseif status >= 400 then
            io.stderr:write(string.format("[tasks_update] Error %d\n", status))
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
    local excluded = {
        {request_categories.backoff, "Backoff"},
        {request_categories.suppressed, "Suppressed thread"},
        {request_categories.fallback, "Fallback"}
    }
    for _, item in ipairs(excluded) do
        if item[1] > 0 then
            io.stderr:write(string.format("[tasks_update] %s requests (excluded): %d\n", item[2], item[1]))
        end
    end
end

local function print_status_distribution(aggregated, lua_total, summary)
    io.write("\n--- tasks_update HTTP Status Distribution (all threads) ---\n")

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

-- 純粋関数: カテゴリ別集計
local function aggregate_categories(categories)
    return {
        executed = categories.executed,
        backoff = categories.backoff,
        suppressed = categories.suppressed,
        fallback = categories.fallback
    }
end

-- 純粋関数: 整合性検証
local function verify_consistency(categories, total_requests)
    local sum = categories.executed + categories.backoff +
                categories.suppressed + categories.fallback
    return sum == total_requests, sum
end

function done(summary, latency, requests)
    -- カテゴリ別集計（純粋関数）
    local categories = aggregate_categories(request_categories)

    -- 整合性検証（純粋関数）
    local is_consistent, sum = verify_consistency(categories, summary.requests)

    -- 副作用: 出力
    if not is_consistent then
        io.stderr:write(string.format(
            "[tasks_update] WARN: Inconsistency detected: total=%d, sum(categories)=%d\n",
            summary.requests, sum))
    end

    -- 副作用: カテゴリ別集計の出力
    io.write("\n--- Request Categories (REQ-MEASURE-401) ---\n")
    io.write(string.format("  Executed:   %d\n", categories.executed))
    io.write(string.format("  Backoff:    %d\n", categories.backoff))
    io.write(string.format("  Suppressed: %d\n", categories.suppressed))
    io.write(string.format("  Fallback:   %d\n", categories.fallback))
    io.write(string.format("  Total:      %d\n", sum))
    io.write(string.format("  Excluded:   %d (backoff + suppressed + fallback)\n",
        categories.backoff + categories.suppressed + categories.fallback))
    io.write("\n")

    print_excluded_requests()
    common.print_summary("tasks_update", summary)

    if error_tracker then
        print_status_distribution(error_tracker.get_all_threads_aggregated_summary(), common.total_requests, summary)
    end

    io.stderr:write(string.format("[tasks_update] Retry config: RETRY_COUNT=%d, BACKOFF_MAX=%d\n", RETRY_COUNT, BACKOFF_MAX))

    local retry_stats = {
        {thread_retry_count, "Thread successful retries"},
        {thread_retry_exhausted_count, "Thread retry exhausted"}
    }
    for _, stat in ipairs(retry_stats) do
        if stat[1] > 0 then
            io.stderr:write(string.format("[tasks_update] %s: %d\n", stat[2], stat[1]))
        end
    end

    common.finalize_benchmark(summary, latency, requests)
end
