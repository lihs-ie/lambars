-- Benchmark: POST /tasks/bulk
-- Tests bulk operations with partial failure handling (207 Multi-Status)

package.path = package.path .. ";scripts/?.lua"
local common = require("common")

local counter = 0
local batch_sizes = {10, 50, 100}  -- Aligned with API BULK_LIMIT

function request()
    counter = counter + 1
    local batch_size = batch_sizes[(counter % #batch_sizes) + 1]

    local tasks = {}
    for i = 1, batch_size do
        if i == 1 and counter % 10 == 0 then
            -- Inject invalid task for partial failure testing
            table.insert(tasks, {
                title = "",
                description = "Invalid task for partial failure testing",
                priority = "low",
                tags = common.array({})
            })
        else
            table.insert(tasks, {
                title = common.random_title() .. " #" .. i,
                description = "Bulk created task " .. i .. " of " .. batch_size,
                priority = common.random_priority(),
                tags = common.array({"bulk", "batch-" .. batch_size})
            })
        end
    end

    local body = common.json_encode({tasks = common.array(tasks)})
    return wrk.format("POST", "/tasks/bulk", {["Content-Type"] = "application/json"}, body)
end

local handlers = common.create_threaded_handlers("tasks_bulk")
setup = handlers.setup
response = handlers.response
done = handlers.done
