-- Traversable (Batch processing) benchmarks
-- benches/api/benchmarks/scripts/traversable.lua
--
-- Endpoints:
--   POST /tasks/validate-batch
--   POST /tasks/fetch-batch
--   POST /tasks/collect-optional
--   POST /tasks/execute-sequential
--   POST /tasks/enrich-batch

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
local request_types = {"validate", "fetch", "collect", "sequential", "enrich"}

function request()
    counter = counter + 1
    local req_type = request_types[(counter % #request_types) + 1]

    if req_type == "validate" then
        -- POST /tasks/validate-batch (use pre-built JSON to avoid encoding issues)
        local priorities = {"low", "medium", "high", "critical"}
        local priority = priorities[(counter % #priorities) + 1]
        local body = '{"tasks":[{"title":"Benchmark Task ' .. counter .. '","description":"Test task","priority":"' .. priority .. '","tags":["benchmark","test"]}]}'
        return wrk.format("POST", "/tasks/validate-batch", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "fetch" then
        -- POST /tasks/fetch-batch
        local task_id_list = {test_ids.get_task_id(1), test_ids.get_task_id(2), test_ids.get_task_id(3)}
        local body = common.json_encode({task_ids = task_id_list})
        return wrk.format("POST", "/tasks/fetch-batch", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "collect" then
        -- POST /tasks/collect-optional
        local task_id_list = {test_ids.get_task_id(1), test_ids.get_task_id(2), test_ids.get_task_id(3)}
        local body = common.json_encode({
            task_ids = task_id_list,
            field = "description"
        })
        return wrk.format("POST", "/tasks/collect-optional", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "sequential" then
        -- POST /tasks/execute-sequential (use pre-built JSON)
        local task_id1 = test_ids.get_task_id(1)
        local task_id2 = test_ids.get_task_id(2)
        local body = '{"operations":[{"type":"update_status","task_id":"' .. task_id1 .. '","new_status":"in_progress"},{"type":"add_tag","task_id":"' .. task_id2 .. '","tag":"benchmark"}]}'
        return wrk.format("POST", "/tasks/execute-sequential", {["Content-Type"] = "application/json"}, body)

    else
        -- POST /tasks/enrich-batch (use pre-built JSON)
        local task_id1 = test_ids.get_task_id(1)
        local task_id2 = test_ids.get_task_id(2)
        local task_id3 = test_ids.get_task_id(3)
        local body = '{"task_ids":["' .. task_id1 .. '","' .. task_id2 .. '","' .. task_id3 .. '"],"include":["project","subtasks"]}'
        return wrk.format("POST", "/tasks/enrich-batch", {["Content-Type"] = "application/json"}, body)
    end
end

function response(status, headers, body)
    common.track_response(status)
    if status >= 400 and status ~= 404 then
        io.stderr:write(string.format("[traversable] Error %d\n", status))
    end
end

function done(summary, latency, requests)
    common.print_summary("traversable", summary)
end
