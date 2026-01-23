-- Endpoint-specific benchmark: PUT /tasks/{id}
-- benches/api/benchmarks/scripts/tasks_update.lua
--
-- Target API features:
--   - Lens (focusing on struct fields for immutable updates)
--   - Prism (pattern matching on sum types)
--   - Optional (Lens + Prism composition for Option<T> fields)
--   - Bifunctor (transforming both success and error sides)
--
-- Demonstrates lambars' optics for type-safe, immutable updates
-- and Either-based error handling with Bifunctor transformations.

package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local test_ids = common.load_test_ids()

local counter = 0

-- Update scenarios to cycle through
local update_types = {"priority", "status", "description", "title", "full"}

function request()
    counter = counter + 1
    local task_id = test_ids.get_task_id(counter)
    local update_type = update_types[(counter % #update_types) + 1]

    local body

    if update_type == "priority" then
        -- Update priority only (Lens focus)
        body = common.json_encode({
            priority = common.random_priority(),
            version = 1
        })
    elseif update_type == "status" then
        -- Update status only (Prism pattern matching)
        body = common.json_encode({
            status = common.random_status(),
            version = 1
        })
    elseif update_type == "description" then
        -- Update optional description (Optional optic)
        body = common.json_encode({
            description = "Updated description via Optional optic - " .. os.time(),
            version = 1
        })
    elseif update_type == "title" then
        -- Update title (Lens focus)
        body = common.json_encode({
            title = common.random_title() .. " (updated)",
            version = 1
        })
    else
        -- Full update (combined optics)
        body = common.json_encode({
            title = common.random_title() .. " (full update)",
            description = "Full update via combined optics",
            priority = common.random_priority(),
            status = common.random_status(),
            tags = common.array({"updated", "benchmark"}),
            version = 1
        })
    end

    return wrk.format("PUT", "/tasks/" .. task_id, {["Content-Type"] = "application/json"}, body)
end

response = common.create_response_handler("tasks_update")
done = common.create_done_handler("tasks_update")
