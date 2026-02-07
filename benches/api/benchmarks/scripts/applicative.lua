-- Applicative (Independent computation combining) benchmarks
-- benches/api/benchmarks/scripts/applicative.lua
--
-- Endpoints:
--   POST /tasks/validate-collect-all
--   GET  /dashboard
--   POST /tasks/build-from-parts
--   POST /tasks/compute-parallel

package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local test_ids = common.load_test_ids()

local counter = 0
local request_types = {"validate", "dashboard", "build", "compute"}

function request()
    counter = counter + 1
    local req_type = request_types[(counter % #request_types) + 1]

    if req_type == "validate" then
        -- POST /tasks/validate-collect-all
        -- deadline must be RFC3339 format (ISO8601 with timezone)
        local body = common.json_encode({
            title = common.random_title(),
            priority = math.random(1, 4),
            deadline = "2025-12-31T23:59:59Z",
            description = "Benchmark task for validation",
            tags = {"benchmark", "test", "applicative"}
        })
        return wrk.format("POST", "/tasks/validate-collect-all", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "dashboard" then
        -- GET /dashboard?include=tasks,projects,stats
        local includes = {"tasks", "projects", "stats", "tasks,projects", "tasks,stats", "all"}
        local include = includes[(counter % #includes) + 1]
        return wrk.format("GET", "/dashboard?include=" .. include)

    elseif req_type == "build" then
        -- POST /tasks/build-from-parts
        -- Uses title_template_id, priority_preset, project_id, use_defaults
        local body = common.json_encode({
            title_template_id = "default-template",
            priority_preset = common.random_priority(),
            project_id = nil,
            use_defaults = true
        })
        return wrk.format("POST", "/tasks/build-from-parts", {["Content-Type"] = "application/json"}, body)

    else
        -- POST /tasks/compute-parallel
        local computation_types = {"complexity", "progress", "dependencies", "estimate"}
        local body = common.json_encode({
            task_id = test_ids.get_task_id(counter),
            computations = {
                computation_types[(counter % #computation_types) + 1],
                computation_types[((counter + 1) % #computation_types) + 1]
            }
        })
        return wrk.format("POST", "/tasks/compute-parallel", {["Content-Type"] = "application/json"}, body)
    end
end

local handlers = common.create_standard_handlers("applicative", {scenario_name = "applicative", output_format = "json"})
function setup(thread) handlers.setup(thread) end
response = handlers.response
done = handlers.done
