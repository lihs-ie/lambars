-- Phase 2.5: Alternative (Fallback and choice) benchmarks
-- benches/api/benchmarks/scripts/phase2_alternative.lua
--
-- Endpoints:
--   GET  /tasks/search-fallback
--   GET  /tasks/{id}/config
--   POST /tasks/filter-conditional
--   POST /tasks/aggregate-sources
--   GET  /tasks/first-available

package.path = package.path .. ";scripts/?.lua"
local common = require("common")

-- Try to load generated test IDs
local test_ids
local ok, ids = pcall(require, "test_ids")
if ok then
    test_ids = ids
else
    test_ids = {
        task_ids = {
            "a1b2c3d4-e5f6-4789-abcd-ef0123456789",
            "b2c3d4e5-f6a7-4890-bcde-f01234567890",
            "c3d4e5f6-a7b8-4901-cdef-012345678901"
        },
        get_task_id = function(index) return test_ids.task_ids[((index - 1) % #test_ids.task_ids) + 1] end
    }
end

local counter = 0
local request_types = {"search", "config", "filter", "aggregate", "first"}

-- Search queries
local search_queries = {"auth", "database", "api", "cache", "test"}

-- Config keys
local config_keys = {"timeout", "retry_count", "max_connections", "log_level"}

function request()
    counter = counter + 1
    local req_type = request_types[(counter % #request_types) + 1]
    local task_id = test_ids.get_task_id(counter)

    if req_type == "search" then
        -- GET /tasks/search-fallback?query=...
        local query = search_queries[(counter % #search_queries) + 1]
        return wrk.format("GET", "/tasks/search-fallback?query=" .. query)

    elseif req_type == "config" then
        -- GET /tasks/{id}/config?key=...
        local key = config_keys[(counter % #config_keys) + 1]
        return wrk.format("GET", "/tasks/" .. task_id .. "/config?key=" .. key)

    elseif req_type == "filter" then
        -- POST /tasks/filter-conditional
        local task_id_list = {test_ids.get_task_id(1), test_ids.get_task_id(2), test_ids.get_task_id(3)}
        local body = common.json_encode({
            task_ids = task_id_list,
            conditions = {
                min_priority = "medium",
                has_description = true
            }
        })
        return wrk.format("POST", "/tasks/filter-conditional", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "aggregate" then
        -- POST /tasks/aggregate-sources
        local body = common.json_encode({
            task_id = task_id,
            sources = {"primary", "secondary", "external"},
            merge_strategy = "prefer_first"
        })
        return wrk.format("POST", "/tasks/aggregate-sources", {["Content-Type"] = "application/json"}, body)

    else
        -- GET /tasks/first-available
        return wrk.format("GET", "/tasks/first-available")
    end
end

function response(status, headers, body)
    common.track_response(status)
    if status >= 400 and status ~= 404 then
        io.stderr:write(string.format("[alternative] Error %d\n", status))
    end
end

function done(summary, latency, requests)
    common.print_summary("phase2_alternative")
end
