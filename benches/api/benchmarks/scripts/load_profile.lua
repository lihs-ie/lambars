-- Load Profile Module for wrk benchmarks
-- benches/api/benchmarks/scripts/load_profile.lua
--
-- Provides RPS control and load shaping functions.
-- Supports profiles: constant, ramp_up_down, burst, step_up
--
-- IMPORTANT: Input Validation
-- This module assumes that all input parameters have been validated by run_benchmark.sh
-- before being passed to wrk. The following invariants are guaranteed by the caller:
--   - burst_multiplier > 0
--   - burst_interval > burst_duration
--   - duration > 0
--   - step_count > 0
--   - All numeric parameters are non-negative integers (where applicable)
-- Edge cases like duration < burst_interval or ramp_up + ramp_down > duration are handled
-- in run_benchmark.sh before wrk is invoked (e.g., by running partial phases or scaling).
--
-- Usage:
--   local load_profile = require("load_profile")
--   load_profile.init({
--       profile = "ramp_up_down",
--       target_rps = 1000,
--       duration_seconds = 60,
--       ramp_up_seconds = 15,
--       ramp_down_seconds = 15,
--       burst_multiplier = 3.0,
--       burst_duration_seconds = 5,
--       step_count = 4
--   })
--
--   -- In request():
--   load_profile.wait_for_slot()  -- Rate limiting
--
--   -- In done():
--   load_profile.print_profile_summary()

local M = {}

-- Profile configuration
M.config = {
    profile = "constant",           -- constant, ramp_up_down, burst, step_up
    target_rps = 100,               -- Target requests per second at peak
    duration_seconds = 60,          -- Total benchmark duration
    ramp_up_seconds = 10,           -- Time to ramp up (for ramp_up_down)
    ramp_down_seconds = 10,         -- Time to ramp down (for ramp_up_down)
    burst_multiplier = 3.0,         -- RPS multiplier during burst (for burst)
    burst_duration_seconds = 5,     -- Duration of each burst (for burst)
    burst_interval_seconds = 20,    -- Interval between bursts (for burst)
    step_count = 4,                 -- Number of steps (for step_up)
    min_rps = 10                    -- Minimum RPS (floor)
}

-- Runtime state
M.start_time = nil
M.request_count = 0
M.last_log_time = 0

-- Initialize the load profile
-- @param options table Configuration options
function M.init(options)
    options = options or {}

    for key, value in pairs(options) do
        if M.config[key] ~= nil then
            M.config[key] = value
        end
    end

    M.start_time = os.time()
    M.request_count = 0
    M.last_log_time = 0

    io.write(string.format("[load_profile] Initialized: profile=%s, target_rps=%d, duration=%ds\n",
        M.config.profile, M.config.target_rps, M.config.duration_seconds))
end

-- Calculate elapsed time in seconds
-- @return number Elapsed time in seconds
function M.elapsed_seconds()
    if not M.start_time then
        M.start_time = os.time()
    end
    return os.time() - M.start_time
end

-- Calculate current target RPS based on profile and elapsed time
-- @return number Current target RPS
function M.current_target_rps()
    local elapsed = M.elapsed_seconds()
    local profile = M.config.profile
    local target = M.config.target_rps
    local duration = M.config.duration_seconds

    if profile == "constant" then
        -- Constant RPS throughout
        return target

    elseif profile == "ramp_up_down" then
        -- Linear ramp up, sustained peak, linear ramp down
        local ramp_up = M.config.ramp_up_seconds
        local ramp_down = M.config.ramp_down_seconds
        local sustain_start = ramp_up
        local sustain_end = duration - ramp_down

        if elapsed < ramp_up then
            -- Ramp up phase: linear increase
            local progress = elapsed / ramp_up
            return math.max(M.config.min_rps, target * progress)
        elseif elapsed < sustain_end then
            -- Sustain phase: constant at target
            return target
        elseif elapsed < duration then
            -- Ramp down phase: linear decrease
            local ramp_progress = (elapsed - sustain_end) / ramp_down
            return math.max(M.config.min_rps, target * (1 - ramp_progress))
        else
            return M.config.min_rps
        end

    elseif profile == "burst" then
        -- Periodic bursts (spikes)
        local interval = M.config.burst_interval_seconds
        local burst_duration = M.config.burst_duration_seconds
        local multiplier = M.config.burst_multiplier
        local base_rps = target / multiplier  -- Base RPS between bursts

        local cycle_position = elapsed % interval
        if cycle_position < burst_duration then
            -- Burst phase: multiplied RPS
            return target
        else
            -- Normal phase: base RPS
            return math.max(M.config.min_rps, base_rps)
        end

    elseif profile == "step_up" then
        -- Step function: gradual steps up
        local steps = M.config.step_count
        local step_duration = duration / steps
        local current_step = math.floor(elapsed / step_duration) + 1
        current_step = math.min(current_step, steps)

        -- Each step increases by equal fraction
        local step_rps = (target - M.config.min_rps) / steps
        return M.config.min_rps + (step_rps * current_step)

    else
        -- Unknown profile, default to constant
        return target
    end
end

-- Get the current phase name (for logging/reporting)
-- @return string Current phase name
function M.current_phase()
    local elapsed = M.elapsed_seconds()
    local profile = M.config.profile
    local duration = M.config.duration_seconds

    if profile == "constant" then
        return "constant"

    elseif profile == "ramp_up_down" then
        local ramp_up = M.config.ramp_up_seconds
        local ramp_down = M.config.ramp_down_seconds
        local sustain_end = duration - ramp_down

        if elapsed < ramp_up then
            return "ramp_up"
        elseif elapsed < sustain_end then
            return "sustain"
        else
            return "ramp_down"
        end

    elseif profile == "burst" then
        local interval = M.config.burst_interval_seconds
        local burst_duration = M.config.burst_duration_seconds
        local cycle_position = elapsed % interval

        if cycle_position < burst_duration then
            return "burst"
        else
            return "normal"
        end

    elseif profile == "step_up" then
        local steps = M.config.step_count
        local step_duration = duration / steps
        local current_step = math.floor(elapsed / step_duration) + 1
        current_step = math.min(current_step, steps)
        return string.format("step_%d_of_%d", current_step, steps)

    else
        return "unknown"
    end
end

-- Simple rate limiting using delay calculation
-- NOTE: wrk does not support true rate limiting in request(),
-- so this returns a calculated delay. Use with wrk's --rate option for actual limiting.
-- @return number Suggested delay in microseconds
function M.calculate_delay_microseconds()
    local target_rps = M.current_target_rps()
    if target_rps <= 0 then
        return 1000000  -- 1 second delay if no target
    end

    -- Delay between requests for single thread
    local delay_seconds = 1.0 / target_rps
    return math.floor(delay_seconds * 1000000)
end

-- Log current status (rate limited to once per second)
function M.log_status()
    local elapsed = M.elapsed_seconds()
    if elapsed - M.last_log_time >= 1 then
        M.last_log_time = elapsed
        io.write(string.format("[load_profile] t=%ds phase=%s target_rps=%d requests=%d\n",
            math.floor(elapsed), M.current_phase(), math.floor(M.current_target_rps()), M.request_count))
    end
end

-- Increment request counter
function M.count_request()
    M.request_count = M.request_count + 1
end

-- Print profile summary (call from done())
-- @param summary table Optional wrk summary object for accurate thread-aggregated counts
-- NOTE: wrk runs multiple threads with separate Lua interpreters.
-- M.request_count only tracks requests from the thread where count_request() was called.
-- For accurate total counts, pass wrk's summary object which aggregates across all threads.
function M.print_profile_summary(summary)
    local elapsed = M.elapsed_seconds()

    -- Use wrk's summary for accurate counts if available, otherwise fall back to local count
    local total_requests
    if summary and summary.requests then
        total_requests = summary.requests
    else
        total_requests = M.request_count
    end

    local actual_rps = total_requests / math.max(1, elapsed)

    io.write("\n--- Load Profile Summary ---\n")
    io.write(string.format("Profile: %s\n", M.config.profile))
    io.write(string.format("Duration: %ds (configured: %ds)\n", math.floor(elapsed), M.config.duration_seconds))
    io.write(string.format("Target RPS: %d\n", M.config.target_rps))
    io.write(string.format("Total requests: %d\n", total_requests))
    io.write(string.format("Actual avg RPS: %.1f\n", actual_rps))

    if M.config.profile == "ramp_up_down" then
        io.write(string.format("Ramp up: %ds, Ramp down: %ds\n",
            M.config.ramp_up_seconds, M.config.ramp_down_seconds))
    elseif M.config.profile == "burst" then
        io.write(string.format("Burst multiplier: %.1fx, Burst duration: %ds, Interval: %ds\n",
            M.config.burst_multiplier, M.config.burst_duration_seconds, M.config.burst_interval_seconds))
    elseif M.config.profile == "step_up" then
        io.write(string.format("Steps: %d\n", M.config.step_count))
    end
end

-- Get profile parameters as a table (for result collection)
-- @return table Profile parameters
function M.get_profile_metadata()
    return {
        profile = M.config.profile,
        target_rps = M.config.target_rps,
        duration_seconds = M.config.duration_seconds,
        min_rps = M.config.min_rps,
        ramp_up_seconds = M.config.profile == "ramp_up_down" and M.config.ramp_up_seconds or nil,
        ramp_down_seconds = M.config.profile == "ramp_up_down" and M.config.ramp_down_seconds or nil,
        burst_multiplier = M.config.profile == "burst" and M.config.burst_multiplier or nil,
        burst_duration_seconds = M.config.profile == "burst" and M.config.burst_duration_seconds or nil,
        burst_interval_seconds = M.config.profile == "burst" and M.config.burst_interval_seconds or nil,
        step_count = M.config.profile == "step_up" and M.config.step_count or nil
    }
end

return M
