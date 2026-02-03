-- Benchmark: PUT /tasks/{id} with conflict retry logic

package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local test_ids = common.load_test_ids()

-- Phase 1: No retry (Phase 2 will add server-side retry if needed)
local RETRY_COUNT = tonumber(os.getenv("RETRY_COUNT")) or 0

local state = "update"
local retry_index = nil
local retry_body = nil
local retry_attempt = 0
local counter = 0
local last_request_index = nil
local last_request_is_update = false
local thread_retry_count = 0
local thread_retry_exhausted_count = 0

local update_types = {"priority", "status", "description", "title", "full"}
local error_tracker = (function()
    local ok, module = pcall(require, "error_tracker")
    return ok and module or nil
end)()

function setup(thread)
    if error_tracker then error_tracker.setup_thread(thread) end

    -- WRK_THREADS is required and must match wrk -t
    local total_threads = tonumber(os.getenv("WRK_THREADS"))
    if not total_threads or total_threads <= 0 then
        io.stderr:write(
            "[tasks_update] ERROR: WRK_THREADS environment variable is required and must be > 0.\n" ..
            "Please set WRK_THREADS to match wrk -t value.\n")
        os.exit(1)
    end

    local pool_size = tonumber(os.getenv("ID_POOL_SIZE")) or 10

    -- Validate ID_POOL_SIZE >= WRK_THREADS
    if pool_size < total_threads then
        io.stderr:write(string.format(
            "[tasks_update] ERROR: ID_POOL_SIZE (%d) < WRK_THREADS (%d). " ..
            "This will cause zero-division or severe contention.\n",
            pool_size, total_threads))
        os.exit(1)
    end

    local ids_per_thread = math.floor(pool_size / total_threads)
    local start_index = thread.id * ids_per_thread

    thread:set("id", thread.id)
    thread:set("id_start", start_index)
    thread:set("id_end", start_index + ids_per_thread - 1)
    thread:set("id_range", ids_per_thread)

    io.write(string.format("[Thread %d] ID range: %d-%d (%d IDs)\n",
        thread.id, start_index, start_index + ids_per_thread - 1, ids_per_thread))
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
    state, retry_index, retry_body, retry_attempt = "update", nil, nil, 0
end

function request()
    if state == "retry_get" then
        local task_id, err = test_ids.get_task_id(retry_index)
        if err then
            io.stderr:write("[tasks_update] Error getting task ID for retry: " .. err .. "\n")
            reset_retry_state()
            state = "fallback"
            last_request_is_update = false
            return wrk.format("GET", "/health")
        end
        last_request_is_update = true
        return wrk.format("GET", "/tasks/" .. task_id, {["Accept"] = "application/json"})

    elseif state == "retry_put" then
        local task_id, err = test_ids.get_task_id(retry_index)
        if err or not retry_body then
            io.stderr:write("[tasks_update] Error in retry_put: " .. (err or "retry_body is nil") .. "\n")
            reset_retry_state()
            state = "fallback"
            last_request_is_update = false
            return wrk.format("GET", "/health")
        end
        last_request_is_update = true
        return wrk.format("PUT", "/tasks/" .. task_id, {["Content-Type"] = "application/json"}, retry_body)

    elseif state == "fallback" then
        state = "update"
    end

    -- Use thread-specific ID range if available
    local id_start = tonumber(wrk.thread:get("id_start")) or 0
    local id_range = tonumber(wrk.thread:get("id_range")) or test_ids.get_task_count()

    local next_counter = counter + 1
    -- Map counter to thread-specific ID range
    local local_index = (next_counter % id_range)
    local global_index = id_start + local_index + 1

    local task_state, err = test_ids.get_task_state(global_index)
    if err then
        io.stderr:write("[tasks_update] Error getting task state: " .. err .. "\n")
        state = "fallback"
        last_request_is_update = false
        return wrk.format("GET", "/health")
    end

    counter = next_counter
    last_request_index = global_index
    last_request_is_update = true

    local update_type = update_types[(counter % #update_types) + 1]
    local body = generate_update_body(update_type, task_state.version, counter)

    return wrk.format("PUT", "/tasks/" .. task_state.id, {["Content-Type"] = "application/json"}, body)
end

function response(status, headers, body)
    common.track_response(status, headers)
    if error_tracker then
        error_tracker.track_thread_response(status)
    end
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
                local update_type = update_types[(retry_index % #update_types) + 1]
                retry_body = generate_update_body(update_type, version, retry_index)
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
                io.stderr:write(string.format("[tasks_update] Retry exhausted after %d attempts (giving up)\n", RETRY_COUNT))
                thread_retry_exhausted_count = thread_retry_exhausted_count + 1
                reset_retry_state()
            else
                io.stderr:write(string.format("[tasks_update] Retry PUT got 409 (attempt %d/%d, retrying)\n", retry_attempt, RETRY_COUNT))
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
                    io.stderr:write("[tasks_update] Conflict detected but retries disabled (RETRY_COUNT=0)\n")
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

    -- Print HTTP status distribution
    if error_tracker then
        local aggregated = error_tracker.get_thread_aggregated_summary()
        local total = summary.requests or 0

        io.write("\n--- tasks_update HTTP Status Distribution ---\n")
        if total > 0 then
            local function print_status(code, count)
                if count > 0 then
                    local percentage = (count / total) * 100
                    io.write(string.format("  %s: %d (%.1f%%)\n", code, count, percentage))
                end
            end

            print_status("200 OK", aggregated.status_200)
            print_status("201 Created", aggregated.status_201)
            print_status("207 Multi-Status", aggregated.status_207)
            print_status("400 Bad Request", aggregated.status_400)
            print_status("404 Not Found", aggregated.status_404)
            print_status("409 Conflict", aggregated.status_409)
            print_status("422 Unprocessable Entity", aggregated.status_422)
            print_status("500 Internal Server Error", aggregated.status_500)
            print_status("502 Bad Gateway", aggregated.status_502)
            print_status("Other Status", aggregated.status_other)

            -- Calculate error rate (4xx + 5xx)
            -- Note: status_other only includes non-listed 4xx/5xx (2xx/3xx are excluded in track_thread_response)
            local errors = aggregated.status_400 + aggregated.status_404 +
                           aggregated.status_409 + aggregated.status_422 +
                           aggregated.status_500 + aggregated.status_502 +
                           aggregated.status_other
            local error_rate = (errors / total) * 100
            io.write(string.format("Error Rate: %.2f%% (%d errors / %d requests)\n", error_rate, errors, total))
        else
            io.write("  No requests completed\n")
        end
    end

    io.stderr:write(string.format("[tasks_update] Retry config: RETRY_COUNT=%d\n", RETRY_COUNT))
    if thread_retry_count > 0 then
        io.stderr:write(string.format("[tasks_update] Thread successful retries: %d (per-thread, not aggregated)\n", thread_retry_count))
    end
    if thread_retry_exhausted_count > 0 then
        io.stderr:write(string.format("[tasks_update] Thread retry exhausted: %d (per-thread, not aggregated)\n", thread_retry_exhausted_count))
    end
end
