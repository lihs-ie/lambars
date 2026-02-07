-- Health endpoint benchmark script
-- benches/api/benchmarks/scripts/health.lua
--
-- Simple health check endpoint for RPS rate control verification.
-- This script sends GET /health requests with minimal overhead,
-- making it ideal for testing rate control accuracy without
-- backend complexity.
--
-- Endpoints:
--   GET /health

package.path = package.path .. ";scripts/?.lua"
local common = require("common")

local handlers = common.create_standard_handlers("health", {scenario_name = "health", output_format = "json"})

function setup(thread)
    handlers.setup(thread)
end

function init(args)
end

-- Simple request function that always hits /health
function request()
    return wrk.format("GET", "/health")
end

-- Standard response handler
response = handlers.response

-- Standard done handler
done = handlers.done
