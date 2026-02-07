-- Endpoint-specific benchmark: GET /tasks/search
-- benches/api/benchmarks/scripts/tasks_search.lua
--
-- Target API features:
--   - PersistentTreeMap (ordered persistent map for indexed search)
--   - Alternative (fallback search strategies)
--
-- Demonstrates lambars' query patterns with Foldable aggregation,
-- Monoid combining, and Traversable filtering.

package.path = package.path .. ";scripts/?.lua"
local common = require("common")

local counter = 0

-- Search queries to cycle through
local search_queries = {
    "auth", "database", "api", "cache", "test",
    "deploy", "fix", "update", "implement", "review"
}

-- Search scopes
local search_scopes = {"title", "tags", "all"}

function request()
    counter = counter + 1

    local query = search_queries[(counter % #search_queries) + 1]
    local scope = search_scopes[(counter % #search_scopes) + 1]

    -- GET /tasks/search?q=<query>&in=<scope>
    local path = "/tasks/search?q=" .. query .. "&in=" .. scope

    return wrk.format("GET", path)
end

local handlers = common.create_standard_handlers("tasks_search", {scenario_name = "tasks_search", output_format = "json"})
function setup(thread) handlers.setup(thread) end
response = handlers.response
done = handlers.done
