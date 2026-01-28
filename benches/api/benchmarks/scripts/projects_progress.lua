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

function request()
    counter = counter + 1
    local project_id = test_ids.get_project_id(counter)

    -- GET /projects/{id}/progress
    return wrk.format("GET", "/projects/" .. project_id .. "/progress")
end

response = common.create_response_handler("projects_progress")
done = common.create_done_handler("projects_progress")
