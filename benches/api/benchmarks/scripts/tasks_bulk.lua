-- Endpoint-specific benchmark: POST /tasks/bulk
-- benches/api/benchmarks/scripts/tasks_bulk.lua
--
-- Target API features:
--   - for_! (list comprehension macro)
--   - Alternative (fallback and choice patterns)
--   - Partial failure handling (207 Multi-Status)
--
-- Demonstrates lambars' bulk operations with partial success handling,
-- Alternative-based fallback strategies for validation and save operations.

package.path = package.path .. ";scripts/?.lua"
local common = require("common")

local counter = 0

-- Batch sizes for testing partial failure scenarios
local batch_sizes = {11, 25, 50}

function request()
    counter = counter + 1
    local batch_size = batch_sizes[(counter % #batch_sizes) + 1]

    -- Create batch of tasks for bulk create
    local tasks = {}
    for i = 1, batch_size do
        -- Occasionally inject an invalid task to test partial failure
        if i == 1 and counter % 10 == 0 then
            -- Invalid task (empty title) - should fail validation
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

    local body = common.json_encode({
        tasks = common.array(tasks)
    })

    return wrk.format("POST", "/tasks/bulk", {["Content-Type"] = "application/json"}, body)
end

response = common.create_response_handler("tasks_bulk")
done = common.create_done_handler("tasks_bulk")
