-- Endpoint-specific benchmark: PUT /tasks/{id}
-- benches/api/benchmarks/scripts/tasks_update.lua
--
-- Target API features:
--   - Lens (focusing on struct fields for immutable updates)
--   - Prism (pattern matching on sum types)
--   - Optional (Lens + Prism composition for Option<T> fields)
--   - Bifunctor (transforming both success and error sides)
--
-- REQ-UPDATE-API-001: PUT /tasks/{id} の契約を厳密化
-- REQ-UPDATE-CONFLICT-001: 衝突時の再取得/再試行を定義
-- REQ-UPDATE-IDS-001: IDとversionをペアで管理
--
-- wrk2 制約と前提条件:
-- - 1スレッド1接続を前提とする（-c と -t を同値に設定）
-- - request() と response() は同じスレッドで順序保証される
-- - 多重インフライトが発生しないことを運用で保証
-- - 再試行は次のリクエストサイクルで GET → PUT のシーケンスで実現
--
-- 使用例:
--   wrk2 -t1 -c1 -d30s -R100 -s scripts/tasks_update.lua http://localhost:3000

package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local test_ids = common.load_test_ids()

-- State machine for retry logic
-- "update": Normal PUT request
-- "retry_get": GET request to fetch latest version (after 409)
-- "retry_put": PUT request with fetched version
-- "fallback": /health fallback request (response should be ignored)
local state = "update"
local retry_index = nil
local retry_body = nil

-- Counter for normal updates (incremented after successful request generation)
local counter = 0
local last_request_index = nil

-- Flag to indicate if last request was a valid update (not fallback)
local last_request_is_update = false

-- Metrics (per-thread, not aggregatable in done())
local thread_retry_count = 0

-- Update scenarios to cycle through
local update_types = {"priority", "status", "description", "title", "full"}

-- Generate update body for a given type
-- Uses counter-derived deterministic values for reproducibility
local function generate_update_body(update_type, version, request_counter)
    local timestamp_str = tostring(request_counter)
    if update_type == "priority" then
        return common.json_encode({
            priority = common.random_priority(),
            version = version
        })
    elseif update_type == "status" then
        return common.json_encode({
            status = common.random_status(),
            version = version
        })
    elseif update_type == "description" then
        return common.json_encode({
            description = "Updated description via Optional optic - request " .. timestamp_str,
            version = version
        })
    elseif update_type == "title" then
        return common.json_encode({
            title = common.random_title() .. " (updated)",
            version = version
        })
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

-- Reset retry state and return to normal update mode
local function reset_retry_state()
    state = "update"
    retry_index = nil
    retry_body = nil
end

function request()
    if state == "retry_get" then
        -- Fetch latest version after 409
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
        -- Retry PUT with fetched version
        local task_id, err = test_ids.get_task_id(retry_index)
        if err then
            io.stderr:write("[tasks_update] Error getting task ID for retry PUT: " .. err .. "\n")
            reset_retry_state()
            state = "fallback"
            last_request_is_update = false
            return wrk.format("GET", "/health")
        end
        if not retry_body then
            io.stderr:write("[tasks_update] retry_body is nil in retry_put state\n")
            reset_retry_state()
            state = "fallback"
            last_request_is_update = false
            return wrk.format("GET", "/health")
        end
        last_request_is_update = true
        return wrk.format("PUT", "/tasks/" .. task_id, {["Content-Type"] = "application/json"}, retry_body)

    elseif state == "fallback" then
        -- Previous request was a fallback, reset to normal update
        state = "update"
        -- Fall through to normal update below

    end

    -- Normal update (state == "update" or after fallback reset)
    local next_counter = counter + 1
    local task_state, err = test_ids.get_task_state(next_counter)
    if err then
        io.stderr:write("[tasks_update] Error getting task state: " .. err .. "\n")
        state = "fallback"
        last_request_is_update = false
        return wrk.format("GET", "/health")
    end

    -- Only increment counter after successful state retrieval
    counter = next_counter
    last_request_index = counter
    last_request_is_update = true

    local update_type = update_types[(counter % #update_types) + 1]
    local body = generate_update_body(update_type, task_state.version, counter)

    return wrk.format("PUT", "/tasks/" .. task_state.id, {["Content-Type"] = "application/json"}, body)
end

function response(status, headers, body)
    common.track_response(status, headers)

    -- Ignore fallback responses (e.g., /health)
    if not last_request_is_update then
        return
    end

    if state == "retry_get" then
        -- Process GET response for retry
        if status == 200 then
            -- Parse version from response
            local version = common.extract_version(body)
            if version then
                local success, err = test_ids.set_version(retry_index, version)
                if not success then
                    io.stderr:write("[tasks_update] Failed to set version: " .. (err or "unknown") .. "\n")
                    reset_retry_state()
                    return
                end
                -- Generate new body with updated version
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
        -- Process retry PUT response
        if status == 200 or status == 201 then
            -- Success! Increment version
            local new_version, err = test_ids.increment_version(retry_index)
            if err then
                io.stderr:write("[tasks_update] Error incrementing version after retry: " .. err .. "\n")
            end
            thread_retry_count = thread_retry_count + 1
        elseif status == 409 then
            io.stderr:write("[tasks_update] Retry PUT also got 409 (giving up)\n")
        else
            io.stderr:write(string.format("[tasks_update] Retry PUT failed with status %d\n", status))
        end
        -- Always reset state after retry_put, regardless of success/failure
        reset_retry_state()

    elseif state == "update" then
        -- Process normal update response
        if status == 200 or status == 201 then
            -- Success! Increment version for next request
            if last_request_index then
                local new_version, err = test_ids.increment_version(last_request_index)
                if err then
                    io.stderr:write("[tasks_update] Error incrementing version: " .. err .. "\n")
                end
            end
        elseif status == 409 then
            -- Version conflict - initiate retry
            if last_request_index then
                retry_index = last_request_index
                state = "retry_get"
            end
        elseif status >= 400 then
            io.stderr:write(string.format("[tasks_update] Error %d\n", status))
        end
    end
end

function done(summary, latency, requests)
    common.print_summary("tasks_update", summary)
    -- Note: thread_retry_count is per-thread and not aggregated
    -- This output is for debugging purposes only
    if thread_retry_count > 0 then
        io.stderr:write(string.format("[tasks_update] Thread retries: %d (per-thread, not aggregated)\n", thread_retry_count))
    end
end
