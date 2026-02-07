-- PersistentTreeMap (Ordered data operations) benchmarks
-- benches/api/benchmarks/scripts/ordered.lua
--
-- Endpoints:
--   GET /tasks/by-deadline
--   GET /tasks/timeline
--   GET /projects/leaderboard

package.path = package.path .. ";scripts/?.lua"
local common = require("common")

local counter = 0
local request_types = {"by_deadline", "timeline", "leaderboard"}
local handlers = common.create_standard_handlers("ordered", {scenario_name = "ordered", output_format = "json"})

function setup(thread)
    handlers.setup(thread)
end

function init(args)
end

-- Date ranges for testing
local date_ranges = {
    {from = "2024-01-01", to = "2024-06-30"},
    {from = "2024-07-01", to = "2024-12-31"},
    {from = "2025-01-01", to = "2025-12-31"}
}

-- Sort orders for timeline
local orders = {"priority_first", "created_first"}

-- Sort criteria for leaderboard
local sort_by_options = {"completed_tasks", "completion_rate", "total_tasks"}

function request()
    counter = counter + 1
    local req_type = request_types[(counter % #request_types) + 1]

    if req_type == "by_deadline" then
        -- GET /tasks/by-deadline?from=...&to=...&limit=50
        local range = date_ranges[(counter % #date_ranges) + 1]
        local path = string.format(
            "/tasks/by-deadline?from=%s&to=%s&limit=50&offset=%d",
            range.from, range.to, (counter % 10) * 10
        )
        return wrk.format("GET", path)

    elseif req_type == "timeline" then
        -- GET /tasks/timeline?order=...&limit=20
        local order = orders[(counter % #orders) + 1]
        local path = string.format(
            "/tasks/timeline?order=%s&limit=20&offset=%d",
            order, (counter % 5) * 20
        )
        return wrk.format("GET", path)

    else
        -- GET /projects/leaderboard?top=10&sort_by=...
        local sort_by = sort_by_options[(counter % #sort_by_options) + 1]
        local path = string.format("/projects/leaderboard?top=10&sort_by=%s", sort_by)
        return wrk.format("GET", path)
    end
end

response = handlers.response
done = handlers.done
