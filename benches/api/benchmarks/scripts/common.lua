-- Common utilities for wrk benchmark scripts
-- benches/api/benchmarks/scripts/common.lua

local M = {}

-- Error tracking
M.status_counts = {
    [200] = 0,
    [201] = 0,
    [400] = 0,
    [404] = 0,
    [422] = 0,
    [500] = 0,
    other = 0
}
M.total_requests = 0

-- Generate a random UUID v4
function M.random_uuid()
    local template = "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx"
    return string.gsub(template, "[xy]", function(c)
        local v = (c == "x") and math.random(0, 0xf) or math.random(8, 0xb)
        return string.format("%x", v)
    end)
end

-- Generate a random task title
function M.random_title()
    local prefixes = {"Implement", "Fix", "Update", "Refactor", "Test", "Deploy", "Review", "Optimize"}
    local subjects = {"authentication", "database", "API", "cache", "logging", "metrics", "UI", "docs"}
    return prefixes[math.random(#prefixes)] .. " " .. subjects[math.random(#subjects)]
end

-- Generate a random priority
function M.random_priority()
    local priorities = {"low", "medium", "high", "critical"}
    return priorities[math.random(#priorities)]
end

-- Generate a random status
function M.random_status()
    local statuses = {"pending", "in_progress", "completed", "cancelled"}
    return statuses[math.random(#statuses)]
end

-- Create an empty array marker
M.EMPTY_ARRAY = setmetatable({}, {__is_array = true})

-- Create an array (ensures proper JSON array encoding)
function M.array(tbl)
    return setmetatable(tbl or {}, {__is_array = true})
end

-- JSON encode a table (simple implementation for benchmark use)
function M.json_encode(tbl)
    if type(tbl) ~= "table" then
        if type(tbl) == "string" then
            return '"' .. tbl:gsub('"', '\\"') .. '"'
        else
            return tostring(tbl)
        end
    end

    local mt = getmetatable(tbl)
    local is_array = (mt and mt.__is_array) or #tbl > 0
    local parts = {}

    if is_array then
        for _, v in ipairs(tbl) do
            table.insert(parts, M.json_encode(v))
        end
        return "[" .. table.concat(parts, ",") .. "]"
    else
        for k, v in pairs(tbl) do
            table.insert(parts, '"' .. k .. '":' .. M.json_encode(v))
        end
        return "{" .. table.concat(parts, ",") .. "}"
    end
end

-- Track response status
function M.track_response(status)
    M.total_requests = M.total_requests + 1
    if M.status_counts[status] then
        M.status_counts[status] = M.status_counts[status] + 1
    else
        M.status_counts.other = M.status_counts.other + 1
    end
end

-- Print status summary (call from done())
function M.print_summary(script_name)
    io.write("\n--- " .. script_name .. " Status Summary ---\n")
    io.write(string.format("Total requests: %d\n", M.total_requests))
    for status, count in pairs(M.status_counts) do
        if count > 0 then
            local pct = (count / M.total_requests) * 100
            io.write(string.format("  %s: %d (%.1f%%)\n", tostring(status), count, pct))
        end
    end
    local error_count = M.status_counts[400] + M.status_counts[404] +
                        M.status_counts[422] + M.status_counts[500] + M.status_counts.other
    local error_rate = (error_count / M.total_requests) * 100
    io.write(string.format("Error rate: %.1f%%\n", error_rate))
end

-- Initialize random seed (with optional SEED environment variable for reproducibility)
local seed = tonumber(os.getenv("SEED")) or os.time()
math.randomseed(seed)

return M
