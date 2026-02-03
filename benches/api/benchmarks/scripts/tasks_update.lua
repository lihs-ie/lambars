-- Benchmark: PUT /tasks/{id} with conflict retry logic

package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local test_ids = common.load_test_ids()

local RETRY_COUNT = tonumber(os.getenv("RETRY_COUNT")) or 0
local state = "update"
local retry_index, retry_body, retry_attempt = nil, nil, 0
local counter = 0
local last_request_index, last_request_is_update = nil, false
local thread_retry_count, thread_retry_exhausted_count = 0, 0
local update_types = {"priority", "status", "description", "title", "full"}
local error_tracker = pcall(require, "error_tracker") and require("error_tracker") or nil

function init(args)
    common.init_benchmark({
        scenario_name = "tasks_update",
        output_format = "json"
    })
end

function setup(thread)
    if error_tracker then error_tracker.setup_thread(thread) end

    -- WRK_THREADS は wrk2 の -t オプションと同じ値を設定することを推奨
    -- 未設定の場合は、wrk2 が自動的にスレッドを割り当てるため、
    -- スレッド数は実行時に決定される（デフォルト: 1）
    local total_threads = tonumber(os.getenv("WRK_THREADS"))
    local pool_size = tonumber(os.getenv("ID_POOL_SIZE")) or 10

    -- WRK_THREADS が未設定の場合、wrk2 のデフォルトスレッド数（1）を仮定
    -- または、thread.id の最大値から推定する（複雑化を避けるため、1 を仮定）
    if not total_threads or total_threads <= 0 then
        total_threads = 1
    end

    if pool_size < total_threads then
        io.stderr:write(string.format(
            "[tasks_update] WARN: ID_POOL_SIZE (%d) < WRK_THREADS (%d), using pool_size=%d\n",
            pool_size, total_threads, pool_size))
        total_threads = pool_size
    end

    local ids_per_thread = math.floor(pool_size / total_threads)
    local thread_id = tonumber(thread.id) or 0
    local start_index = thread_id * ids_per_thread

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
end

function request()
    if state == "retry_get" then
        local task_id, err = test_ids.get_task_id(retry_index)
        if err then
            io.stderr:write("[tasks_update] Error getting task ID for retry: " .. err .. "\n")
            reset_retry_state()
            state = "fallback"
            last_request_is_update = false
            if wrk and wrk.format then
                return wrk.format("GET", "/health")
            else
                return ""
            end
        end
        last_request_is_update = true
        if wrk and wrk.format then
            return wrk.format("GET", "/tasks/" .. task_id, {["Accept"] = "application/json"})
        else
            return ""
        end
    elseif state == "retry_put" then
        local task_id, err = test_ids.get_task_id(retry_index)
        if err or not retry_body then
            io.stderr:write("[tasks_update] Error in retry_put: " .. (err or "retry_body is nil") .. "\n")
            reset_retry_state()
            state = "fallback"
            last_request_is_update = false
            if wrk and wrk.format then
                return wrk.format("GET", "/health")
            else
                return ""
            end
        end
        last_request_is_update = true
        if wrk and wrk.format then
            return wrk.format("PUT", "/tasks/" .. task_id, {["Content-Type"] = "application/json"}, retry_body)
        else
            return ""
        end
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
        state = "fallback"
        last_request_is_update = false
        if wrk and wrk.format then
            return wrk.format("GET", "/health")
        else
            return ""
        end
    end

    last_request_index = global_index
    last_request_is_update = true

    local update_type = update_types[(counter % #update_types) + 1]
    local body = generate_update_body(update_type, task_state.version, counter)

    if wrk and wrk.format then
        return wrk.format("PUT", "/tasks/" .. task_state.id, {["Content-Type"] = "application/json"}, body)
    else
        return ""
    end
end

function response(status, headers, body)
    if not status then return end
    common.track_response(status, headers)
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
                    state = "retry_get"
                end
            end
        elseif status >= 400 then
            io.stderr:write(string.format("[tasks_update] Error %d\n", status))
        end
    end
end

function done(summary, latency, requests)
    common.print_summary("tasks_update", summary)

    if error_tracker then
        local aggregated = error_tracker.get_thread_aggregated_summary()
        local total = summary.requests or 0

        io.write("\n--- tasks_update HTTP Status Distribution ---\n")
        if total > 0 then
            local status_codes = {
                {"200 OK", aggregated.status_200},
                {"201 Created", aggregated.status_201},
                {"207 Multi-Status", aggregated.status_207},
                {"400 Bad Request", aggregated.status_400},
                {"404 Not Found", aggregated.status_404},
                {"409 Conflict", aggregated.status_409},
                {"422 Unprocessable Entity", aggregated.status_422},
                {"500 Internal Server Error", aggregated.status_500},
                {"502 Bad Gateway", aggregated.status_502},
                {"Other Status", aggregated.status_other}
            }

            for _, pair in ipairs(status_codes) do
                local code, count = pair[1], pair[2]
                if count > 0 then
                    io.write(string.format("  %s: %d (%.1f%%)\n", code, count, (count / total) * 100))
                end
            end

            local errors = aggregated.status_400 + aggregated.status_404 + aggregated.status_409 +
                           aggregated.status_422 + aggregated.status_500 + aggregated.status_502 +
                           aggregated.status_other
            io.write(string.format("Error Rate: %.2f%% (%d errors / %d requests)\n", (errors / total) * 100, errors, total))
        else
            io.write("  No requests completed\n")
        end
    end

    io.stderr:write(string.format("[tasks_update] Retry config: RETRY_COUNT=%d\n", RETRY_COUNT))
    if thread_retry_count > 0 then
        io.stderr:write(string.format("[tasks_update] Thread successful retries: %d\n", thread_retry_count))
    end
    if thread_retry_exhausted_count > 0 then
        io.stderr:write(string.format("[tasks_update] Thread retry exhausted: %d\n", thread_retry_exhausted_count))
    end

    common.finalize_benchmark(summary, latency, requests)
end
