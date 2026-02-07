-- Endpoint-specific benchmark: GET /projects/{id}/progress
-- benches/api/benchmarks/scripts/projects_progress.lua
--
-- Target API features:
--   - Foldable (aggregating task counts with fold operations)
--   - Trampoline (stack-safe recursion for deep project hierarchies)
--   - Monoid (combining progress counters with empty and combine)
--
-- Demonstrates lambars' functional aggregation patterns using
-- Foldable to accumulate task statistics, Trampoline for stack-safe
-- recursive traversal, and Monoid for composable result combination.

package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local test_ids = common.load_test_ids()

local counter = 0
local handlers = common.create_standard_handlers("projects_progress", {scenario_name = "projects_progress", output_format = "json"})

function setup(thread)
    handlers.setup(thread)
end

function init(args)
end

function request()
    counter = counter + 1
    local project_id = test_ids.get_project_id(counter)

    -- GET /projects/{id}/progress
    return wrk.format("GET", "/projects/" .. project_id .. "/progress")
end

response = handlers.response
done = handlers.done
