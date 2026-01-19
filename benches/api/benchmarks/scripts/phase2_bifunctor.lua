-- Phase 2.7: Bifunctor (Two-parameter type transformations) benchmarks
-- benches/api/benchmarks/scripts/phase2_bifunctor.lua
--
-- Endpoints:
--   POST /tasks/process-with-error-transform
--   POST /tasks/transform-pair
--   POST /tasks/enrich-error
--   POST /tasks/convert-error-domain
--   POST /tasks/batch-transform-results

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
local request_types = {"process", "pair", "enrich", "convert", "batch"}

function request()
    counter = counter + 1
    local req_type = request_types[(counter % #request_types) + 1]
    local task_id = test_ids.get_task_id(counter)

    if req_type == "process" then
        -- POST /tasks/process-with-error-transform
        local body = common.json_encode({
            task_id = task_id,
            processing_options = {
                validate = true,
                simulate_failure = (counter % 5 == 0)
            }
        })
        return wrk.format("POST", "/tasks/process-with-error-transform", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "pair" then
        -- POST /tasks/transform-pair
        local body = common.json_encode({
            task_id = task_id,
            metadata = {
                source = "benchmark",
                attributes = {environment = "test", iteration = tostring(counter)}
            },
            transform_option = "both"
        })
        return wrk.format("POST", "/tasks/transform-pair", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "enrich" then
        -- POST /tasks/enrich-error
        local body = common.json_encode({
            task_id = task_id,
            include_trace = true,
            simulate_failure = (counter % 3 == 0)
        })
        return wrk.format("POST", "/tasks/enrich-error", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "convert" then
        -- POST /tasks/convert-error-domain
        -- Requires operation and data fields
        local body = common.json_encode({
            operation = "create",
            data = {
                title = common.random_title(),
                description = "Benchmark task for error domain conversion"
            },
            simulate_error = (counter % 4 == 0) and "not_found" or nil
        })
        return wrk.format("POST", "/tasks/convert-error-domain", {["Content-Type"] = "application/json"}, body)

    else
        -- POST /tasks/batch-transform-results
        -- Uses task_ids and fail_ids to control success/failure
        local task_id_list = {test_ids.get_task_id(1), test_ids.get_task_id(2), test_ids.get_task_id(3)}
        local fail_list = (counter % 2 == 0) and {test_ids.get_task_id(1)} or common.array({})
        local body = common.json_encode({
            task_ids = task_id_list,
            fail_ids = fail_list
        })
        return wrk.format("POST", "/tasks/batch-transform-results", {["Content-Type"] = "application/json"}, body)
    end
end

function response(status, headers, body)
    common.track_response(status)
    if status >= 400 and status ~= 404 then
        io.stderr:write(string.format("[bifunctor] Error %d\n", status))
    end
end

function done(summary, latency, requests)
    common.print_summary("phase2_bifunctor", summary)
end
