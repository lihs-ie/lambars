-- Data seeding script for benchmark scenarios
-- benches/api/benchmarks/scripts/seed_data.lua
--
-- Seeds benchmark data using the payload_generator module.
-- Supports configurable data scales, random seeds, and incremental seeding.
--
-- Usage:
--   lua seed_data.lua
--
-- Environment Variables:
--   DATA_SCALE       - Data scale preset (small, medium, large)
--   RECORD_COUNT     - Explicit record count (overrides scale default)
--   RANDOM_SEED      - Random seed for reproducibility
--   INCREMENTAL      - Set to "1" for incremental seeding
--   API_ENDPOINT     - API endpoint (default: http://localhost:8080)
--   BATCH_SIZE       - Records per batch (default: 100)
--   PAYLOAD_VARIANT  - Payload variant (minimal, standard, complex, heavy)
--
-- Exit codes:
--   0 - Success
--   1 - Configuration error
--   2 - Seeding error

local payload_generator = require("payload_generator")

local M = {}

-- Shell-safe string escaping function
-- Wraps the string in single quotes and escapes any internal single quotes
-- This prevents shell injection attacks when passing untrusted input to shell commands
-- @param str string|nil The string to escape
-- @return string The shell-safe escaped string
local function shell_escape(str)
    if str == nil then
        return "''"
    end
    -- Escape single quotes by ending the single-quoted string,
    -- adding an escaped single quote, and starting a new single-quoted string
    -- Example: "it's" becomes 'it'\''s'
    return "'" .. string.gsub(tostring(str), "'", "'\\''") .. "'"
end

-- Default configuration
M.config = {
    scale = "medium",
    record_count = nil,
    seed = nil,
    incremental = false,
    endpoint = "http://localhost:8080",
    batch_size = 100,
    payload_variant = "standard",
}

-- Scale to default record count mapping
M.scale_defaults = {
    small = 1000,
    medium = 10000,
    large = 1000000,
}

-- Initialize configuration from environment variables
function M.init_config()
    local config = M.config

    -- DATA_SCALE
    local scale = os.getenv("DATA_SCALE")
    if scale and (scale == "small" or scale == "medium" or scale == "large") then
        config.scale = scale
    end

    -- RECORD_COUNT
    local count = os.getenv("RECORD_COUNT")
    if count and tonumber(count) then
        config.record_count = tonumber(count)
    end

    -- RANDOM_SEED
    local seed = os.getenv("RANDOM_SEED")
    if seed and tonumber(seed) then
        config.seed = tonumber(seed)
    end

    -- INCREMENTAL
    local incremental = os.getenv("INCREMENTAL")
    if incremental == "1" or incremental == "true" then
        config.incremental = true
    end

    -- API_ENDPOINT
    local endpoint = os.getenv("API_ENDPOINT")
    if endpoint and endpoint ~= "" then
        config.endpoint = endpoint
    end

    -- BATCH_SIZE
    local batch_size = os.getenv("BATCH_SIZE")
    if batch_size and tonumber(batch_size) then
        config.batch_size = tonumber(batch_size)
    end

    -- PAYLOAD_VARIANT
    local variant = os.getenv("PAYLOAD_VARIANT")
    if variant then
        config.payload_variant = variant
    end

    return config
end

-- Get effective record count
function M.get_effective_record_count(config)
    if config.record_count then
        return config.record_count
    end
    return M.scale_defaults[config.scale] or M.scale_defaults.medium
end

-- Initialize random seed if provided
function M.init_random_seed(config)
    if config.seed then
        math.randomseed(config.seed)
        io.write(string.format("[seed_data] Random seed initialized: %d\n", config.seed))
    else
        -- Use current time for random seed
        math.randomseed(os.time())
        io.write("[seed_data] Random seed: current time\n")
    end
end

-- Print configuration summary
function M.print_config(config)
    local record_count = M.get_effective_record_count(config)

    io.write("\n=== Data Seeding Configuration ===\n")
    io.write(string.format("Scale:          %s\n", config.scale))
    io.write(string.format("Record count:   %d\n", record_count))
    io.write(string.format("Seed:           %s\n", config.seed and tostring(config.seed) or "random"))
    io.write(string.format("Incremental:    %s\n", config.incremental and "yes" or "no"))
    io.write(string.format("Endpoint:       %s\n", config.endpoint))
    io.write(string.format("Batch size:     %d\n", config.batch_size))
    io.write(string.format("Payload variant: %s\n", config.payload_variant))
    io.write("==================================\n\n")
end

-- JSON encode a table (simple implementation for batch payload)
local function json_encode(value)
    if type(value) ~= "table" then
        if type(value) == "string" then
            return '"' .. value:gsub('\\', '\\\\'):gsub('"', '\\"'):gsub('\n', '\\n'):gsub('\r', '\\r'):gsub('\t', '\\t') .. '"'
        elseif type(value) == "boolean" then
            return value and "true" or "false"
        elseif value == nil then
            return "null"
        else
            return tostring(value)
        end
    end

    local metatable = getmetatable(value)
    local is_array = (metatable and metatable.__is_array) or #value > 0
    local parts = {}

    if is_array then
        for _, element in ipairs(value) do
            table.insert(parts, json_encode(element))
        end
        return "[" .. table.concat(parts, ",") .. "]"
    else
        for key, element in pairs(value) do
            table.insert(parts, '"' .. key .. '":' .. json_encode(element))
        end
        return "{" .. table.concat(parts, ",") .. "}"
    end
end

-- Create an array marker for JSON encoding
local function array(table_value)
    return setmetatable(table_value or {}, {__is_array = true})
end

-- Execute HTTP POST request using curl
-- Uses shell_escape for URL and temporary file for body to prevent command injection
-- @param url string The URL to POST to
-- @param body string The JSON body
-- @param headers table Optional headers
-- @return boolean success
-- @return string response body or error message
-- @return number HTTP status code (0 if curl failed)
function M.http_post(url, body, headers)
    -- Escape URL to prevent shell injection
    local escaped_url = shell_escape(url)

    -- Build header arguments with proper escaping
    local header_arguments = ""
    for key, value in pairs(headers or {}) do
        -- Use shell_escape for header values
        local escaped_header = shell_escape(key .. ": " .. value)
        header_arguments = header_arguments .. " -H " .. escaped_header
    end

    -- Write body to temporary file to avoid shell escaping issues
    -- This is safer than embedding the body in the command string
    local temp_body_file = os.tmpname()
    local body_file_handle = io.open(temp_body_file, "w")
    if not body_file_handle then
        return false, "Failed to create temporary body file", 0
    end
    body_file_handle:write(body)
    body_file_handle:close()

    -- Build curl command with:
    -- -s: silent mode
    -- -w: write out HTTP status code
    -- -o: output response body to temp file
    -- --data-binary @file: read body from file (avoids shell escaping issues)
    local temp_response_file = os.tmpname()
    local command = string.format(
        "curl -s -w '%%{http_code}' -X POST%s --data-binary @%s %s -o %s 2>&1",
        header_arguments,
        shell_escape(temp_body_file),
        escaped_url,
        shell_escape(temp_response_file)
    )

    local handle = io.popen(command)
    if not handle then
        os.remove(temp_body_file)
        os.remove(temp_response_file)
        return false, "Failed to execute curl command", 0
    end

    local status_code_string = handle:read("*a")
    local close_success = handle:close()

    -- Clean up body temp file
    os.remove(temp_body_file)

    -- Read response body from temp file
    local response_body = ""
    local response_file_handle = io.open(temp_response_file, "r")
    if response_file_handle then
        response_body = response_file_handle:read("*a")
        response_file_handle:close()
    end
    os.remove(temp_response_file)

    if not close_success then
        return false, "curl command failed: " .. (response_body or "unknown error"), 0
    end

    local status_code = tonumber(status_code_string) or 0

    if status_code >= 200 and status_code < 300 then
        return true, response_body, status_code
    else
        return false, response_body, status_code
    end
end

-- Execute HTTP DELETE request using curl
-- Uses shell_escape for URL to prevent command injection
-- @param url string The URL to DELETE
-- @return boolean success
-- @return string response body or error message
-- @return number HTTP status code
function M.http_delete(url)
    -- Escape URL to prevent shell injection
    local escaped_url = shell_escape(url)

    local temp_response_file = os.tmpname()
    local command = string.format(
        "curl -s -w '%%{http_code}' -X DELETE %s -o %s 2>&1",
        escaped_url,
        shell_escape(temp_response_file)
    )

    local handle = io.popen(command)
    if not handle then
        os.remove(temp_response_file)
        return false, "Failed to execute curl command", 0
    end

    local status_code_string = handle:read("*a")
    local close_success = handle:close()

    -- Read response body from temp file
    local response_body = ""
    local response_file_handle = io.open(temp_response_file, "r")
    if response_file_handle then
        response_body = response_file_handle:read("*a")
        response_file_handle:close()
    end
    os.remove(temp_response_file)

    if not close_success then
        return false, "curl command failed: " .. (response_body or "unknown error"), 0
    end

    local status_code = tonumber(status_code_string) or 0

    if status_code >= 200 and status_code < 300 then
        return true, response_body, status_code
    else
        return false, response_body, status_code
    end
end

-- Clear existing data from the API
-- @param endpoint string The API base endpoint
-- @return boolean success
function M.clear_existing_data(endpoint)
    io.write("[seed_data] Clearing existing data...\n")
    local success, response, status_code = M.http_delete(endpoint .. "/admin/clear")

    if success then
        io.write("[seed_data] Data cleared successfully\n")
        return true
    elseif status_code == 404 then
        io.write("[seed_data] Warning: Clear endpoint not found (404), continuing...\n")
        return true
    else
        io.stderr:write(string.format("[seed_data] Warning: Failed to clear data (HTTP %d): %s\n", status_code, response))
        return false
    end
end

-- Generate a batch of task objects (not JSON strings)
-- @param batch_size number Number of tasks to generate
-- @return table Array of task objects
function M.generate_batch(batch_size)
    local batch = {}
    local config = payload_generator.get_config()

    for index = 1, batch_size do
        -- Generate task object directly instead of using create_task()
        -- which returns JSON string
        local title_prefixes = {"Implement", "Fix", "Update", "Refactor", "Test", "Deploy", "Review", "Optimize", "Create", "Delete"}
        local title_subjects = {"authentication", "database", "API", "cache", "logging", "metrics", "UI", "docs", "tests", "config"}
        local priorities = {"low", "medium", "high", "critical"}

        -- Generate title
        local title = title_prefixes[math.random(#title_prefixes)] .. " " ..
                      title_subjects[math.random(#title_subjects)] .. " " ..
                      tostring(math.random(1000, 9999))

        -- Generate description
        local description_charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 "
        local description_parts = {}
        for character_index = 1, config.description_length do
            local character_position = math.random(1, #description_charset)
            description_parts[character_index] = description_charset:sub(character_position, character_position)
        end
        local description = table.concat(description_parts)

        -- Generate tags
        local tags = {}
        local tag_prefixes = {"feature", "bug", "enhancement", "docs", "test", "perf", "security", "ui", "backend", "infra"}
        for tag_index = 1, config.tag_count do
            local prefix = tag_prefixes[((tag_index - 1) % #tag_prefixes) + 1]
            tags[tag_index] = string.format("%s-%d", prefix, tag_index)
        end

        -- Generate subtasks
        local subtasks = {}
        for subtask_index = 1, config.subtask_count do
            local subtask_title_parts = {}
            for character_index = 1, 20 do
                local character_position = math.random(1, #description_charset)
                subtask_title_parts[character_index] = description_charset:sub(character_position, character_position)
            end
            subtasks[subtask_index] = {
                id = string.format("%08x-%04x-4%03x-%04x-%012x",
                    math.random(0, 0xffffffff),
                    math.random(0, 0xffff),
                    math.random(0, 0xfff),
                    math.random(0x8000, 0xbfff),
                    math.random(0, 0xffffffffffff)),
                title = string.format("Subtask %d: %s", subtask_index, table.concat(subtask_title_parts)),
                completed = math.random() > 0.5
            }
        end

        batch[index] = {
            title = title,
            description = description,
            priority = priorities[math.random(#priorities)],
            tags = array(tags),
            subtasks = array(subtasks)
        }
    end

    return batch
end

-- Seed tasks by sending HTTP requests to the API
-- @param config table Configuration
-- @return number Number of successfully seeded records, or -1 on fatal error
function M.seed_tasks(config)
    local record_count = M.get_effective_record_count(config)
    local batch_size = config.batch_size
    local total_batches = math.ceil(record_count / batch_size)
    local endpoint = config.endpoint

    io.write(string.format("[seed_data] Starting to seed %d records in %d batches...\n",
        record_count, total_batches))

    local seeded = 0
    local errors = 0
    local consecutive_errors = 0
    local maximum_consecutive_errors = 5

    for batch_number = 1, total_batches do
        local current_batch_size = math.min(batch_size, record_count - seeded)
        local batch = M.generate_batch(current_batch_size)

        -- Create batch payload with tasks wrapper
        local batch_payload = json_encode({tasks = array(batch)})

        -- Send batch to API
        local success, response, status_code = M.http_post(
            endpoint .. "/tasks/bulk",
            batch_payload,
            {["Content-Type"] = "application/json"}
        )

        if success then
            seeded = seeded + current_batch_size
            consecutive_errors = 0
        else
            errors = errors + 1
            consecutive_errors = consecutive_errors + 1
            io.stderr:write(string.format("[seed_data] Error seeding batch %d (HTTP %d): %s\n",
                batch_number, status_code, response:sub(1, 200)))

            -- Abort if too many consecutive errors
            if consecutive_errors >= maximum_consecutive_errors then
                io.stderr:write(string.format("[seed_data] FATAL: %d consecutive errors, aborting\n",
                    maximum_consecutive_errors))
                return -1
            end
        end

        -- Progress report every 10 batches or at the end
        if batch_number % 10 == 0 or batch_number == total_batches then
            local progress = (seeded / record_count) * 100
            io.write(string.format("[seed_data] Progress: %d/%d records (%.1f%%), errors: %d\n",
                seeded, record_count, progress, errors))
        end
    end

    io.write(string.format("[seed_data] Seeding complete: %d records created, %d errors\n", seeded, errors))

    if errors > 0 then
        io.stderr:write(string.format("[seed_data] WARNING: %d batches failed to insert\n", errors))
    end

    return seeded
end

-- Main entry point
function M.run()
    -- Initialize configuration
    local config = M.init_config()

    -- Set payload variant
    payload_generator.set_variant(config.payload_variant)

    -- Initialize random seed
    M.init_random_seed(config)

    -- Print configuration
    M.print_config(config)

    -- Clear existing data if not incremental
    if not config.incremental then
        local clear_success = M.clear_existing_data(config.endpoint)
        if not clear_success then
            io.stderr:write("[seed_data] Warning: Could not clear existing data, continuing anyway...\n")
        end
    else
        io.write("[seed_data] Incremental mode: preserving existing data\n")
    end

    -- Run seeding
    local seeded = M.seed_tasks(config)

    -- Exit with error code if seeding failed
    if seeded < 0 then
        os.exit(2)
    end

    return seeded
end

-- Export for use as a module
return M
