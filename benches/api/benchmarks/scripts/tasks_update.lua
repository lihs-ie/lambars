-- PUT /tasks/{id} benchmark with conflict retry logic
-- Implements REQ-UPDATE-API-001, REQ-UPDATE-CONFLICT-001, REQ-UPDATE-IDS-001

package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local test_ids = common.load_test_ids()

local RETRY_COUNT = tonumber(os.getenv("RETRY_COUNT")) or 1

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

local function is_retry_exhausted()
    return retry_attempt >= RETRY_COUNT
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

    local next_counter = counter + 1
    local task_state, err = test_ids.get_task_state(next_counter)
    if err then
        io.stderr:write("[tasks_update] Error getting task state: " .. err .. "\n")
        state = "fallback"
        last_request_is_update = false
        return wrk.format("GET", "/health")
    end

    counter = next_counter
    last_request_index = counter
    last_request_is_update = true

    local update_type = update_types[(counter % #update_types) + 1]
    local body = generate_update_body(update_type, task_state.version, counter)

    return wrk.format("PUT", "/tasks/" .. task_state.id, {["Content-Type"] = "application/json"}, body)
end

function response(status, headers, body)
    common.track_response(status, headers)
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
            if is_retry_exhausted() then
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
                if is_retry_exhausted() then
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
    io.stderr:write(string.format("[tasks_update] Retry config: RETRY_COUNT=%d\n", RETRY_COUNT))
    if thread_retry_count > 0 then
        io.stderr:write(string.format("[tasks_update] Thread successful retries: %d (per-thread, not aggregated)\n", thread_retry_count))
    end
    if thread_retry_exhausted_count > 0 then
        io.stderr:write(string.format("[tasks_update] Thread retry exhausted: %d (per-thread, not aggregated)\n", thread_retry_exhausted_count))
    end
end
