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

response = common.create_response_handler("tasks_search")
done = common.create_done_handler("tasks_search")
