-- Phase 2.10: Misc (partial!, ConcurrentLazy, PersistentDeque, Sum/Product, Freer) benchmarks
-- benches/api/benchmarks/scripts/phase2_misc.lua
--
-- Endpoints:
--   POST /tasks/partial-apply
--   POST /tasks/concurrent-lazy
--   POST /tasks/deque-operations
--   GET  /tasks/aggregate-numeric
--   POST /tasks/freer-workflow

package.path = package.path .. ";scripts/?.lua"
local common = require("common")

-- Try to load generated test IDs, fall back to placeholder UUIDs
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
local request_types = {"partial", "lazy", "deque", "aggregate", "freer"}

function request()
    counter = counter + 1
    local req_type = request_types[(counter % #request_types) + 1]

    if req_type == "partial" then
        -- POST /tasks/partial-apply
        local operations = {"score", "transform", "filter"}
        local task_id_list = {test_ids.get_task_id(1), test_ids.get_task_id(2), test_ids.get_task_id(3)}
        local body = common.json_encode({
            task_ids = task_id_list,
            config = {
                multiplier = 2,
                weights = {
                    complexity_weight = 0.3,
                    priority_weight = 0.5,
                    urgency_weight = 0.2
                },
                flags = {"benchmark"}
            },
            operation = operations[(counter % #operations) + 1]
        })
        return wrk.format("POST", "/tasks/partial-apply", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "lazy" then
        -- POST /tasks/concurrent-lazy
        local body = common.json_encode({
            subsequent_calls = 3
        })
        return wrk.format("POST", "/tasks/concurrent-lazy", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "deque" then
        -- POST /tasks/deque-operations
        local body = common.json_encode({
            operations = {
                {type = "push_front", task_id = test_ids.get_task_id(1)},
                {type = "push_back", task_id = test_ids.get_task_id(2)},
                {type = "pop_front"},
                {type = "peek_back"}
            },
            initial_state = {test_ids.get_task_id(3)}
        })
        return wrk.format("POST", "/tasks/deque-operations", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "aggregate" then
        -- GET /tasks/aggregate-numeric
        local fields = {"priority", "tag_count", "title_length"}
        local aggregations = {"sum", "product", "average"}
        local field = fields[(counter % #fields) + 1]
        local aggregation = aggregations[(counter % #aggregations) + 1]
        local path = string.format("/tasks/aggregate-numeric?field=%s&aggregation=%s", field, aggregation)
        return wrk.format("GET", path)

    else
        -- POST /tasks/freer-workflow
        local body = common.json_encode({
            steps = {
                {type = "create_task", title = common.random_title()},
                {type = "update_priority", task_id = test_ids.get_task_id(1), priority = "high"},
                {type = "add_tag", task_id = test_ids.get_task_id(1), tag = "freer-test"}
            },
            execution_mode = "dry_run"
        })
        return wrk.format("POST", "/tasks/freer-workflow", {["Content-Type"] = "application/json"}, body)
    end
end

function response(status, headers, body)
    common.track_response(status)
    if status >= 400 and status ~= 404 then
        io.stderr:write(string.format("[misc] Error %d\n", status))
    end
end

function done(summary, latency, requests)
    common.print_summary("phase2_misc", summary)
end
