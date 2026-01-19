-- Phase 2.6: pipe_async! (Async pipeline) benchmarks
-- benches/api/benchmarks/scripts/phase2_async_pipeline.lua
--
-- Endpoints:
--   POST /tasks/{id}/transform-async
--   POST /tasks/workflow-async
--   POST /tasks/batch-process-async
--   POST /tasks/{id}/conditional-pipeline

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
local request_types = {"transform", "workflow", "batch", "conditional"}

function request()
    counter = counter + 1
    local req_type = request_types[(counter % #request_types) + 1]
    local task_id = test_ids.get_task_id(counter)

    if req_type == "transform" then
        -- POST /tasks/{id}/transform-async
        -- TransformType enum: normalize_title, bump_priority, lower_priority, add_tag, set_description
        local body = '{"transforms":["normalize_title","bump_priority",{"add_tag":{"tag":"processed"}}],"validate_first":true}'
        return wrk.format("POST", "/tasks/" .. task_id .. "/transform-async", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "workflow" then
        -- POST /tasks/workflow-async
        local body = common.json_encode({
            title = common.random_title(),
            description = "Benchmark workflow task",
            priority = common.random_priority(),
            notify = false
        })
        return wrk.format("POST", "/tasks/workflow-async", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "batch" then
        -- POST /tasks/batch-process-async
        local task_id_list = {test_ids.get_task_id(1), test_ids.get_task_id(2), test_ids.get_task_id(3)}
        local body = common.json_encode({
            task_ids = task_id_list,
            processing_steps = {
                {name = "validate", step_type = "validate"},
                {name = "transform", step_type = "transform"}
            }
        })
        return wrk.format("POST", "/tasks/batch-process-async", {["Content-Type"] = "application/json"}, body)

    else
        -- POST /tasks/{id}/conditional-pipeline
        -- Requires conditions wrapper with PipelineConditions
        local body = common.json_encode({
            conditions = {
                high_priority_threshold = "high",
                simulate_overdue = false
            }
        })
        return wrk.format("POST", "/tasks/" .. task_id .. "/conditional-pipeline", {["Content-Type"] = "application/json"}, body)
    end
end

function response(status, headers, body)
    common.track_response(status)
    if status >= 400 and status ~= 404 then
        io.stderr:write(string.format("[async_pipeline] Error %d\n", status))
    end
end

function done(summary, latency, requests)
    common.print_summary("phase2_async_pipeline", summary)
end
