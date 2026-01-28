-- Endpoint-specific benchmark: POST /tasks-eff
-- benches/api/benchmarks/scripts/tasks_eff.lua
--
-- Target API features:
--   - AsyncIO (asynchronous effect monad)
--   - eff_async! (do-notation macro for effect composition)
--   - ExceptT (monad transformer for error handling)
--
-- Demonstrates lambars' effect system for HTTP handlers with automatic error
-- short-circuiting and clean separation of pure computation and effects.

package.path = package.path .. ";scripts/?.lua"
local common = require("common")

local counter = 0

function request()
    counter = counter + 1

    -- Create task payload for POST /tasks-eff
    local body = common.json_encode({
        title = common.random_title(),
        description = "Benchmark task created via effect system (eff_async! + ExceptT)",
        priority = common.random_priority(),
        tags = common.array({"benchmark", "effect-system", "asyncio"})
    })

    return wrk.format("POST", "/tasks-eff", {["Content-Type"] = "application/json"}, body)
end

response = common.create_response_handler("tasks_eff")
done = common.create_done_handler("tasks_eff")
