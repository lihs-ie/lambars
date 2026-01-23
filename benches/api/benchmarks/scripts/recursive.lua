-- Trampoline (Stack-safe recursion) benchmarks
-- benches/api/benchmarks/scripts/recursive.lua
--
-- Endpoints:
--   GET  /tasks/{id}/flatten-subtasks
--   POST /tasks/resolve-dependencies
--   GET  /projects/{id}/aggregate-tree

package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local test_ids = common.load_test_ids()

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

response = common.create_response_handler("recursive")
done = common.create_done_handler("recursive")
