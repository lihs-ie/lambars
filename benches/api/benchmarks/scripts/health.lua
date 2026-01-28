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

-- Simple request function that always hits /health
function request()
    return wrk.format("GET", "/health")
end

-- Standard response handler
response = common.create_response_handler("health")

-- Standard done handler
done = common.create_done_handler("health")
