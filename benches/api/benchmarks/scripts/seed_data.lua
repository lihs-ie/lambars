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
--
-- Note on TOCTOU: os.tmpname() has an inherent TOCTOU race condition in Lua.
-- The filename is generated but not atomically created. This is a known Lua limitation.
-- We mitigate by using the file immediately and cleaning up on failure.
--
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

    -- Check write success
    local write_success, write_error = body_file_handle:write(body)
    if not write_success then
        body_file_handle:close()
        os.remove(temp_body_file)
        return false, "Failed to write body to temp file: " .. (write_error or "unknown"), 0
    end

    -- Check close success (flush may fail)
    local close_body_success, close_body_error = body_file_handle:close()
    if not close_body_success then
        os.remove(temp_body_file)
        return false, "Failed to close body temp file: " .. (close_body_error or "unknown"), 0
    end

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
--
-- Note on TOCTOU: os.tmpname() has an inherent TOCTOU race condition in Lua.
-- See http_post for details.
--
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

-- =============================================================================
-- REQ-UPDATE-IDS-001: test_ids.lua 生成機能
-- =============================================================================

-- Execute HTTP GET request using curl
--
-- Note on TOCTOU: os.tmpname() has an inherent TOCTOU race condition in Lua.
-- See http_post for details.
--
-- @param url string The URL to GET
-- @return boolean success
-- @return string response body or error message
-- @return number HTTP status code
function M.http_get(url)
    local escaped_url = shell_escape(url)
    local temp_response_file = os.tmpname()
    local command = string.format(
        "curl -s -w '%%{http_code}' -X GET %s -o %s 2>&1",
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

-- Find the start of a JSON array for a specific key at the top level only
-- Only matches keys at depth 1 (inside the root object) to avoid nested key matches.
-- @param json_body string JSON response
-- @param key string The key to find (e.g., "tasks", "projects")
-- @return number|nil Start position of the array content (after '['), or nil if not found
local function find_array_start(json_body, key)
    local position = 1
    local json_length = #json_body
    local depth = 0  -- 0 = outside root, 1 = inside root object, 2+ = nested
    local key_pattern = '"' .. key .. '"%s*:%s*%['

    while position <= json_length do
        local char = json_body:sub(position, position)

        if char == "{" then
            depth = depth + 1
        elseif char == "}" then
            depth = depth - 1
        elseif char == "[" then
            depth = depth + 1
        elseif char == "]" then
            depth = depth - 1
        elseif char == '"' then
            -- We're at a string start
            -- Only check for key match at depth 1 (inside root object)
            if depth == 1 then
                local match_start, match_end = json_body:find(key_pattern, position)
                if match_start == position then
                    -- Found the key at top level, return position after '['
                    -- match_end points to '[', so +1 gives position after it
                    return match_end + 1
                end
            end
            -- Skip string content
            position = position + 1
            while position <= json_length do
                local string_char = json_body:sub(position, position)
                if string_char == '"' then
                    break
                elseif string_char == "\\" then
                    position = position + 1
                end
                position = position + 1
            end
        end
        position = position + 1
    end
    return nil
end

-- Extract id and version from a task object at depth 0 only
-- Tracks brace/bracket depth to ensure we only match top-level fields
-- @param task_object string The task object JSON (including outer braces)
-- @return string|nil task_id
-- @return number|nil task_version
local function extract_id_version_at_depth_zero(task_object)
    local task_id = nil
    local task_version = nil

    local position = 1
    local json_length = #task_object
    local depth = 0  -- Depth relative to the task object (0 = top level fields)

    while position <= json_length do
        local char = task_object:sub(position, position)

        if char == "{" then
            depth = depth + 1
        elseif char == "}" then
            depth = depth - 1
        elseif char == "[" then
            depth = depth + 1
        elseif char == "]" then
            depth = depth - 1
        elseif char == '"' then
            -- We're at a string start
            -- Check if we're at depth 1 (top level of task object) and this is "id" or "version"
            if depth == 1 then
                -- Check for "id"
                local id_match_start, id_match_end, id_value = task_object:find('^"id"%s*:%s*"([^"]+)"', position)
                if id_match_start then
                    task_id = id_value
                    position = id_match_end
                else
                    -- Check for "version"
                    local version_match_start, version_match_end, version_value = task_object:find('^"version"%s*:%s*(%d+)', position)
                    if version_match_start then
                        task_version = tonumber(version_value)
                        position = version_match_end
                    else
                        -- Skip this string
                        position = position + 1
                        while position <= json_length do
                            local string_char = task_object:sub(position, position)
                            if string_char == '"' then
                                break
                            elseif string_char == "\\" then
                                position = position + 1
                            end
                            position = position + 1
                        end
                    end
                end
            else
                -- Skip string at non-zero depth
                position = position + 1
                while position <= json_length do
                    local string_char = task_object:sub(position, position)
                    if string_char == '"' then
                        break
                    elseif string_char == "\\" then
                        position = position + 1
                    end
                    position = position + 1
                end
            end
        end
        position = position + 1

        -- Early exit if both found
        if task_id and task_version then
            break
        end
    end

    return task_id, task_version
end

-- Extract task IDs and versions from JSON response (top-level tasks only)
-- This function carefully parses JSON to avoid matching nested subtask id/version pairs.
-- It tracks brace depth to ensure only depth-0 fields are matched.
--
-- @param json_body string JSON response from GET /tasks
-- @return table Array of { id = string, version = number }
local function extract_task_states(json_body)
    local states = {}

    -- Response format is: { "tasks": [ { task1 }, { task2 }, ... ] }
    -- Only accept the expected format - no fallback to avoid parsing wrong arrays
    local array_content_start = find_array_start(json_body, "tasks")
    if not array_content_start then
        return states
    end

    local position = array_content_start
    local json_length = #json_body

    while position <= json_length do
        -- Skip whitespace
        while position <= json_length and json_body:sub(position, position):match("%s") do
            position = position + 1
        end

        if position > json_length then
            break
        end

        -- Check for array end
        if json_body:sub(position, position) == "]" then
            break
        end

        -- Skip comma between objects
        if json_body:sub(position, position) == "," then
            position = position + 1
            while position <= json_length and json_body:sub(position, position):match("%s") do
                position = position + 1
            end
        end

        if position > json_length then
            break
        end

        -- Expect opening brace of task object
        if json_body:sub(position, position) ~= "{" then
            position = position + 1
        else
            -- Found a task object start, now find its end by tracking brace depth
            local object_start = position
            local depth = 1
            position = position + 1

            while position <= json_length and depth > 0 do
                local char = json_body:sub(position, position)
                if char == "{" then
                    depth = depth + 1
                elseif char == "}" then
                    depth = depth - 1
                elseif char == '"' then
                    -- Skip string content to avoid false brace matches
                    position = position + 1
                    while position <= json_length do
                        local string_char = json_body:sub(position, position)
                        if string_char == '"' then
                            break
                        elseif string_char == "\\" then
                            position = position + 1
                        end
                        position = position + 1
                    end
                end
                position = position + 1
            end

            local object_end = position - 1
            local task_object = json_body:sub(object_start, object_end)

            -- Extract id and version at depth 0 only
            local task_id, task_version = extract_id_version_at_depth_zero(task_object)

            -- Only add if both id and version were found at top level
            if task_id and task_version then
                table.insert(states, { id = task_id, version = task_version })
            end
        end
    end

    return states
end

-- Extract project IDs from JSON response (top-level projects only)
-- Similar to extract_task_states but only extracts id field
-- @param json_body string JSON response from GET /projects
-- @return table Array of project ID strings
local function extract_project_ids(json_body)
    local ids = {}

    -- Response format is: { "projects": [ { project1 }, { project2 }, ... ] }
    -- Only accept the expected format - no fallback to avoid parsing wrong arrays
    local array_content_start = find_array_start(json_body, "projects")
    if not array_content_start then
        return ids
    end

    local position = array_content_start
    local json_length = #json_body

    while position <= json_length do
        while position <= json_length and json_body:sub(position, position):match("%s") do
            position = position + 1
        end

        if position > json_length then
            break
        end

        if json_body:sub(position, position) == "]" then
            break
        end

        if json_body:sub(position, position) == "," then
            position = position + 1
            while position <= json_length and json_body:sub(position, position):match("%s") do
                position = position + 1
            end
        end

        if position > json_length then
            break
        end

        if json_body:sub(position, position) ~= "{" then
            position = position + 1
        else
            local object_start = position
            local depth = 1
            position = position + 1

            while position <= json_length and depth > 0 do
                local char = json_body:sub(position, position)
                if char == "{" then
                    depth = depth + 1
                elseif char == "}" then
                    depth = depth - 1
                elseif char == '"' then
                    position = position + 1
                    while position <= json_length do
                        local string_char = json_body:sub(position, position)
                        if string_char == '"' then
                            break
                        elseif string_char == "\\" then
                            position = position + 1
                        end
                        position = position + 1
                    end
                end
                position = position + 1
            end

            local object_end = position - 1
            local project_object = json_body:sub(object_start, object_end)

            -- Extract id at depth 0 only (reuse the same logic)
            local project_id, _ = extract_id_version_at_depth_zero(project_object)

            if project_id then
                table.insert(ids, project_id)
            end
        end
    end

    return ids
end

-- Generate test_ids.lua from seeded tasks
-- @param endpoint string API endpoint
-- @param count number Number of task IDs to fetch (default 10)
-- @return boolean success
function M.generate_test_ids(endpoint, count)
    count = count or 10
    io.write(string.format("[seed_data] Generating test_ids.lua with %d task IDs...\n", count))

    -- Fetch tasks from API
    local success, response, status_code = M.http_get(
        endpoint .. "/tasks?limit=" .. count .. "&offset=0"
    )

    if not success then
        io.stderr:write(string.format("[seed_data] Failed to fetch tasks (HTTP %d): %s\n",
            status_code, response:sub(1, 200)))
        return false
    end

    -- Extract task states from response
    local task_states = extract_task_states(response)

    if #task_states == 0 then
        io.stderr:write("[seed_data] No tasks found in response\n")
        return false
    end

    -- Also fetch projects for project_ids using the extract_project_ids function
    local project_ids_list = {}
    local project_success, project_response, project_status = M.http_get(
        endpoint .. "/projects?limit=3&offset=0"
    )

    if project_success then
        project_ids_list = extract_project_ids(project_response)
    end

    -- Fallback project IDs if none found (ensures get_project_id never fails due to empty array)
    local fallback_project_ids = {
        "00000000-0000-4000-8000-000000000001",
        "00000000-0000-4000-8000-000000000002",
        "00000000-0000-4000-8000-000000000003",
    }
    if #project_ids_list == 0 then
        io.stderr:write("[seed_data] Warning: No projects found, using fallback IDs\n")
        project_ids_list = fallback_project_ids
    end

    -- Generate test_ids.lua content
    -- Get script directory with nil fallback
    local debug_info = debug.getinfo(1, "S")
    local source_path = debug_info and debug_info.source or "@./seed_data.lua"
    local script_dir = source_path:match("@(.*/)")
    if not script_dir then
        -- Fallback to current directory if pattern doesn't match
        script_dir = "./"
    end
    local output_path = script_dir .. "test_ids.lua"

    local file = io.open(output_path, "w")
    if not file then
        io.stderr:write("[seed_data] Failed to open test_ids.lua for writing\n")
        return false
    end

    -- Helper function to safely write to file with error checking
    local function safe_write(content)
        local success, err = file:write(content)
        if not success then
            io.stderr:write(string.format("[seed_data] Failed to write to file: %s\n", err or "unknown error"))
            return false
        end
        return true
    end

    -- Write header
    if not safe_write("-- Auto-generated test IDs for benchmarking\n") then file:close(); return false end
    if not safe_write("-- Generated at: " .. os.date() .. "\n") then file:close(); return false end
    if not safe_write("-- API URL: " .. endpoint .. "\n") then file:close(); return false end
    if not safe_write("--\n") then file:close(); return false end
    if not safe_write("-- REQ-UPDATE-IDS-001: ID と version をペアで管理する\n") then file:close(); return false end
    if not safe_write("-- - get_task_state(index) で ID/version ペアを取得\n") then file:close(); return false end
    if not safe_write("-- - increment_version(index) で更新成功時に version をインクリメント\n") then file:close(); return false end
    if not safe_write("-- - reset_versions() でベンチマーク開始時に全 version を 1 にリセット\n") then file:close(); return false end
    if not safe_write("--\n") then file:close(); return false end
    if not safe_write("-- wrk2 スレッドモデルに関する注意:\n") then file:close(); return false end
    if not safe_write("-- 各スレッドは独立した Lua 状態を持つため、同一 ID を複数スレッドで\n") then file:close(); return false end
    if not safe_write("-- 更新すると、Lua 側の version がスレッド間で分岐する。\n") then file:close(); return false end
    if not safe_write("-- 低衝突シナリオ（single writer）では問題ないが、高衝突シナリオでは\n") then file:close(); return false end
    if not safe_write("-- サーバ側の version との不整合が発生する可能性がある。\n") then file:close(); return false end
    if not safe_write("-- 高衝突シナリオでは、409 発生時に GET で最新 version を取得して再試行する。\n") then file:close(); return false end
    if not safe_write("--\n") then file:close(); return false end
    if not safe_write("-- 状態変更に関する注意:\n") then file:close(); return false end
    if not safe_write("-- increment_version / set_version / reset_versions は内部状態を破壊的に変更する。\n") then file:close(); return false end
    if not safe_write("-- これは wrk2 のスレッドモデルで各スレッドが独立した状態を持つ必要があるため。\n") then file:close(); return false end
    if not safe_write("-- 関数型プログラミングの純粋性は犠牲になるが、ベンチマークスクリプトの設計上必要。\n") then file:close(); return false end
    if not safe_write("\n") then file:close(); return false end
    if not safe_write("local M = {}\n\n") then file:close(); return false end

    -- Write task_states with proper string escaping using %q format specifier
    if not safe_write("-- Task states: ID と version をペアで管理（内部データ）\n") then file:close(); return false end
    if not safe_write("local task_states = {\n") then file:close(); return false end
    for _, state in ipairs(task_states) do
        -- Use %q for proper Lua string escaping (handles special characters)
        local line = string.format("    { id = %q, version = %d },\n", state.id, state.version)
        if not safe_write(line) then file:close(); return false end
    end
    if not safe_write("}\n\n") then file:close(); return false end

    -- Write project_ids with proper string escaping
    if not safe_write("local project_ids = {\n") then file:close(); return false end
    for _, id in ipairs(project_ids_list) do
        -- Use %q for proper Lua string escaping
        local line = string.format("    %q,\n", id)
        if not safe_write(line) then file:close(); return false end
    end
    if not safe_write("}\n\n") then file:close(); return false end

    -- Write helper functions (same as Phase 1)
    local helper_functions = [[
-- インデックスを正規化し、検証する
-- @param index 1-based index
-- @param count 配列の要素数
-- @return 正規化されたインデックス（1-based）、または nil とエラーメッセージ
local function normalize_index(index, count)
    if type(index) ~= "number" then
        return nil, "index must be a number, got " .. type(index)
    end
    if index ~= math.floor(index) then
        return nil, "index must be an integer, got " .. tostring(index)
    end
    if count == 0 then
        return nil, "cannot index into empty array"
    end
    local normalized = ((index - 1) % count) + 1
    return normalized, nil
end

-- version の妥当性を検証する
-- @param version バージョン番号
-- @return true if valid, false and error message otherwise
local function validate_version(version)
    if type(version) ~= "number" then
        return false, "version must be a number, got " .. type(version)
    end
    if version ~= math.floor(version) then
        return false, "version must be an integer, got " .. tostring(version)
    end
    if version < 1 then
        return false, "version must be >= 1, got " .. tostring(version)
    end
    return true, nil
end

function M.get_task_id(index)
    local actual_index, err = normalize_index(index, #task_states)
    if err then
        return nil, err
    end
    return task_states[actual_index].id, nil
end

function M.get_project_id(index)
    local actual_index, err = normalize_index(index, #project_ids)
    if err then
        return nil, err
    end
    return project_ids[actual_index], nil
end

function M.get_task_state(index)
    local actual_index, err = normalize_index(index, #task_states)
    if err then
        return nil, err
    end
    local state = task_states[actual_index]
    return { id = state.id, version = state.version }, nil
end

function M.increment_version(index)
    local actual_index, err = normalize_index(index, #task_states)
    if err then
        return nil, err
    end
    task_states[actual_index].version = task_states[actual_index].version + 1
    return task_states[actual_index].version, nil
end

function M.set_version(index, version)
    local actual_index, index_err = normalize_index(index, #task_states)
    if index_err then
        return nil, index_err
    end
    local valid, version_err = validate_version(version)
    if not valid then
        return nil, version_err
    end
    task_states[actual_index].version = version
    return true, nil
end

function M.reset_versions()
    for i = 1, #task_states do
        task_states[i].version = 1
    end
end

function M.get_task_count()
    return #task_states
end

function M.get_project_count()
    return #project_ids
end

function M.get_all_task_ids()
    local copy = {}
    for i, state in ipairs(task_states) do
        copy[i] = state.id
    end
    return copy
end

function M.get_all_project_ids()
    local copy = {}
    for i, id in ipairs(project_ids) do
        copy[i] = id
    end
    return copy
end

return M
]]
    if not safe_write(helper_functions) then file:close(); return false end

    file:close()

    io.write(string.format("[seed_data] Generated test_ids.lua with %d task IDs and %d project IDs\n",
        #task_states, #project_ids_list))

    return true
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

    -- REQ-UPDATE-IDS-001: Generate test_ids.lua with seeded task IDs
    local test_ids_generated = M.generate_test_ids(config.endpoint, 10)
    if not test_ids_generated then
        io.stderr:write("[seed_data] Warning: Failed to generate test_ids.lua\n")
    end

    return seeded
end

-- Export for use as a module
return M
