-- Benchmark: POST /tasks/bulk

package.path = package.path .. ";scripts/?.lua"
local common = require("common")

local counter = 0
local batch_sizes = {10, 50, 100}

function request()
    counter = counter + 1
    local batch_size = batch_sizes[(counter % #batch_sizes) + 1]

    local tasks = {}
    for i = 1, batch_size do
        local task = (i == 1 and counter % 10 == 0) and {
            title = "",
            description = "Invalid task for partial failure testing",
            priority = "low",
            tags = common.array({})
        } or {
            title = common.random_title() .. " #" .. i,
            description = "Bulk created task " .. i .. " of " .. batch_size,
            priority = common.random_priority(),
            tags = common.array({"bulk", "batch-" .. batch_size})
        }
        table.insert(tasks, task)
    end

    local body = common.json_encode({tasks = common.array(tasks)})
    return wrk.format("POST", "/tasks/bulk", {["Content-Type"] = "application/json"}, body)
end

function init(args)
    common.init_benchmark({
        scenario_name = "tasks_bulk",
        output_format = "json"
    })
end

local handlers = common.create_threaded_handlers("tasks_bulk")
setup = handlers.setup
response = handlers.response

function done(summary, latency, requests)
    handlers.done(summary, latency, requests)
    common.finalize_benchmark(summary, latency, requests)
end
