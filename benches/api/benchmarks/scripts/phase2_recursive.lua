-- Phase 2.2: Trampoline (Stack-safe recursion) benchmarks
-- benches/api/benchmarks/scripts/phase2_recursive.lua
--
-- Endpoints:
--   GET  /tasks/{id}/flatten-subtasks
--   POST /tasks/resolve-dependencies
--   GET  /projects/{id}/aggregate-tree

package.path = package.path .. ";scripts/?.lua"
local common = require("common")

-- Try to load generated test IDs, fall back to placeholder UUIDs
local test_ids
local ok, ids = pcall(require, "test_ids")
if ok then
    test_ids = ids
else
    -- Fallback placeholder IDs (will return 404 but still test the endpoint)
    test_ids = {
        task_ids = {
            "a1b2c3d4-e5f6-4789-abcd-ef0123456789",
            "b2c3d4e5-f6a7-4890-bcde-f01234567890",
            "c3d4e5f6-a7b8-4901-cdef-012345678901"
        },
        project_ids = {
            "f6a7b8c9-d0e1-4234-fabc-345678901234",
            "a7b8c9d0-e1f2-4345-abcd-456789012345"
        },
        get_task_id = function(index) return test_ids.task_ids[((index - 1) % #test_ids.task_ids) + 1] end,
        get_project_id = function(index) return test_ids.project_ids[((index - 1) % #test_ids.project_ids) + 1] end
    }
end

local counter = 0
local request_types = {"flatten", "dependencies", "aggregate"}

function request()
    counter = counter + 1
    local req_type = request_types[(counter % #request_types) + 1]
    local task_id = test_ids.get_task_id(counter)
    local project_id = test_ids.get_project_id(counter)

    if req_type == "flatten" then
        -- GET /tasks/{id}/flatten-subtasks?max_depth=100
        return wrk.format("GET", "/tasks/" .. task_id .. "/flatten-subtasks?max_depth=100")

    elseif req_type == "dependencies" then
        -- POST /tasks/resolve-dependencies
        local body = common.json_encode({
            task_ids = {task_id, test_ids.get_task_id(counter + 1)}
        })
        return wrk.format("POST", "/tasks/resolve-dependencies", {["Content-Type"] = "application/json"}, body)

    else
        -- GET /projects/{id}/aggregate-tree?max_depth=10
        return wrk.format("GET", "/projects/" .. project_id .. "/aggregate-tree?max_depth=10")
    end
end

function response(status, headers, body)
    common.track_response(status)
    if status >= 400 and status ~= 404 then
        io.stderr:write(string.format("[recursive] Error %d\n", status))
    end
end

function done(summary, latency, requests)
    common.print_summary("phase2_recursive")
end
