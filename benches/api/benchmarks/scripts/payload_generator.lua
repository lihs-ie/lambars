-- Payload Generator Module for wrk benchmarks
-- benches/api/benchmarks/scripts/payload_generator.lua
--
-- Generates task payloads with varying complexity for benchmarks.
-- Supports variants: minimal, standard, complex, heavy
--
-- Payload sizes (estimated based on JSON structure):
--   minimal:  0 tags, 0 subtasks, desc 20 chars   (~200 bytes)
--   standard: 10 tags, 10 subtasks, desc 200 chars (~2 KB / 2,000 bytes)
--   complex:  100 tags, 50 subtasks, desc 1000 chars (~10 KB / 10,000 bytes)
--   heavy:    100 tags, 200 subtasks, desc 5000 chars (~25 KB / 25,000 bytes)
--
-- Size breakdown for heavy variant:
--   - Base JSON structure: ~100 bytes
--   - Title (200 chars): ~200 bytes
--   - Description (5000 chars): ~5,000 bytes
--   - Tags (100 items, avg ~15 bytes each): ~1,500 bytes
--   - Subtasks (200 items, avg ~90 bytes each): ~18,000 bytes
--   Total: ~25,000 bytes
--
-- Usage:
--   local payload_generator = require("payload_generator")
--   payload_generator.set_variant("complex")
--
--   -- Generate payloads:
--   local create_body = payload_generator.create_task()
--   local update_body = payload_generator.update_task()

local M = {}

-- Current payload variant
M.current_variant = "standard"

-- Payload variant configurations
M.variants = {
    minimal = {
        tag_count = 0,
        subtask_count = 0,
        description_length = 20,
        title_length = 30
    },
    standard = {
        tag_count = 10,
        subtask_count = 10,
        description_length = 200,
        title_length = 50
    },
    complex = {
        tag_count = 100,
        subtask_count = 50,
        description_length = 1000,
        title_length = 100
    },
    heavy = {
        tag_count = 100,
        subtask_count = 200,
        description_length = 5000,
        title_length = 200
    }
}

-- Estimated payload sizes (bytes)
-- Based on actual JSON structure calculations
M.estimated_sizes = {
    minimal = 200,      -- ~200 bytes
    standard = 2000,    -- ~2 KB
    complex = 10000,    -- ~10 KB
    heavy = 25000       -- ~25 KB
}

-- Set the current payload variant
-- @param variant string One of: minimal, standard, complex, heavy
function M.set_variant(variant)
    if M.variants[variant] then
        M.current_variant = variant
    else
        io.stderr:write(string.format("[payload_generator] Unknown variant '%s', using 'standard'\n", variant))
        M.current_variant = "standard"
    end
end

-- Get current variant configuration
-- @return table Variant configuration
function M.get_config()
    return M.variants[M.current_variant]
end

-- Get estimated payload size
-- @return number Estimated size in bytes
function M.get_estimated_size()
    return M.estimated_sizes[M.current_variant] or M.estimated_sizes.standard
end

-- Generate a random string of specified length
-- @param length number Length of the string
-- @return string Random alphanumeric string
local function random_string(length)
    local charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 "
    local result = {}
    for i = 1, length do
        local index = math.random(1, #charset)
        result[i] = charset:sub(index, index)
    end
    return table.concat(result)
end

-- Generate a random UUID v4
-- @return string UUID string
local function random_uuid()
    local template = "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx"
    return string.gsub(template, "[xy]", function(c)
        local v = (c == "x") and math.random(0, 0xf) or math.random(8, 0xb)
        return string.format("%x", v)
    end)
end

-- Generate a random task title
-- @return string Random title
local function random_title(length)
    local prefixes = {"Implement", "Fix", "Update", "Refactor", "Test", "Deploy", "Review", "Optimize", "Create", "Delete"}
    local subjects = {"authentication", "database", "API", "cache", "logging", "metrics", "UI", "docs", "tests", "config"}
    local base = prefixes[math.random(#prefixes)] .. " " .. subjects[math.random(#subjects)]

    if #base >= length then
        return base:sub(1, length)
    else
        return base .. " " .. random_string(length - #base - 1)
    end
end

-- Generate a random priority
-- @return string Priority value
local function random_priority()
    local priorities = {"low", "medium", "high", "critical"}
    return priorities[math.random(#priorities)]
end

-- Generate a random status
-- @return string Status value
local function random_status()
    local statuses = {"pending", "in_progress", "completed", "cancelled"}
    return statuses[math.random(#statuses)]
end

-- Generate tags array
-- @param count number Number of tags to generate
-- @return table Array of tag strings
local function generate_tags(count)
    local tags = {}
    local tag_prefixes = {"feature", "bug", "enhancement", "docs", "test", "perf", "security", "ui", "backend", "infra"}

    for i = 1, count do
        local prefix = tag_prefixes[((i - 1) % #tag_prefixes) + 1]
        tags[i] = string.format("%s-%d", prefix, i)
    end

    return tags
end

-- Generate subtasks array
-- @param count number Number of subtasks to generate
-- @return table Array of subtask objects
local function generate_subtasks(count)
    local subtasks = {}

    for i = 1, count do
        subtasks[i] = {
            id = random_uuid(),
            title = string.format("Subtask %d: %s", i, random_string(20)),
            completed = math.random() > 0.5
        }
    end

    return subtasks
end

-- JSON encode a table (simple implementation)
local function json_encode(tbl)
    if type(tbl) ~= "table" then
        if type(tbl) == "string" then
            return '"' .. tbl:gsub('\\', '\\\\'):gsub('"', '\\"'):gsub('\n', '\\n') .. '"'
        elseif type(tbl) == "boolean" then
            return tbl and "true" or "false"
        else
            return tostring(tbl)
        end
    end

    local mt = getmetatable(tbl)
    local is_array = (mt and mt.__is_array) or #tbl > 0
    local parts = {}

    if is_array then
        for _, v in ipairs(tbl) do
            table.insert(parts, json_encode(v))
        end
        return "[" .. table.concat(parts, ",") .. "]"
    else
        for k, v in pairs(tbl) do
            table.insert(parts, '"' .. k .. '":' .. json_encode(v))
        end
        return "{" .. table.concat(parts, ",") .. "}"
    end
end

-- Create an array marker for JSON encoding
local function array(tbl)
    return setmetatable(tbl or {}, {__is_array = true})
end

-- Generate a CREATE task payload
-- @param options table Optional overrides
-- @return string JSON payload
function M.create_task(options)
    options = options or {}
    local config = M.get_config()

    local payload = {
        title = options.title or random_title(config.title_length),
        description = options.description or random_string(config.description_length),
        priority = options.priority or random_priority(),
        tags = array(options.tags or generate_tags(config.tag_count)),
        subtasks = array(options.subtasks or generate_subtasks(config.subtask_count))
    }

    return json_encode(payload)
end

-- Generate an UPDATE task payload
-- @param options table Optional overrides
-- @return string JSON payload
function M.update_task(options)
    options = options or {}
    local config = M.get_config()

    -- Update payload typically has fewer fields
    local payload = {
        title = options.title or random_title(config.title_length),
        description = options.description or random_string(config.description_length),
        priority = options.priority or random_priority(),
        status = options.status or random_status()
    }

    -- Optionally include tags (50% chance)
    if math.random() > 0.5 or options.tags then
        payload.tags = array(options.tags or generate_tags(math.floor(config.tag_count / 2)))
    end

    return json_encode(payload)
end

-- Generate a PATCH task payload (partial update)
-- @param options table Optional overrides
-- @return string JSON payload
function M.patch_task(options)
    options = options or {}

    -- Patch only updates one or two fields
    local fields = {"title", "description", "priority", "status"}
    local selected_field = fields[math.random(#fields)]

    local payload = {}

    if selected_field == "title" then
        payload.title = options.title or random_title(30)
    elseif selected_field == "description" then
        payload.description = options.description or random_string(100)
    elseif selected_field == "priority" then
        payload.priority = options.priority or random_priority()
    elseif selected_field == "status" then
        payload.status = options.status or random_status()
    end

    return json_encode(payload)
end

-- Generate a BATCH create payload
-- @param count number Number of tasks in batch
-- @param options table Optional overrides for each task
-- @return string JSON payload
function M.batch_create(count, options)
    count = count or 10
    options = options or {}

    local tasks = {}
    for i = 1, count do
        local config = M.get_config()
        tasks[i] = {
            title = string.format("Batch Task %d: %s", i, random_title(config.title_length - 15)),
            description = random_string(config.description_length),
            priority = random_priority(),
            tags = array(generate_tags(math.floor(config.tag_count / 2))),
            subtasks = array(generate_subtasks(math.floor(config.subtask_count / 2)))
        }
    end

    return json_encode({tasks = array(tasks)})
end

-- Generate a SEARCH payload
-- @param options table Search criteria
-- @return string JSON payload
function M.search_tasks(options)
    options = options or {}

    local payload = {}

    if options.query then
        payload.query = options.query
    else
        payload.query = random_string(10)
    end

    if options.priority then
        payload.priority = options.priority
    elseif math.random() > 0.5 then
        payload.priority = random_priority()
    end

    if options.status then
        payload.status = options.status
    elseif math.random() > 0.5 then
        payload.status = random_status()
    end

    if options.limit then
        payload.limit = options.limit
    else
        payload.limit = math.random(10, 100)
    end

    if options.offset then
        payload.offset = options.offset
    else
        payload.offset = 0
    end

    return json_encode(payload)
end

-- Get payload variant metadata (for result collection)
-- @return table Payload metadata
function M.get_metadata()
    local config = M.get_config()
    return {
        variant = M.current_variant,
        tag_count = config.tag_count,
        subtask_count = config.subtask_count,
        description_length = config.description_length,
        title_length = config.title_length,
        estimated_size_bytes = M.get_estimated_size()
    }
end

-- Print payload summary
function M.print_summary()
    local config = M.get_config()
    io.write("\n--- Payload Configuration ---\n")
    io.write(string.format("Variant: %s\n", M.current_variant))
    io.write(string.format("Tag count: %d\n", config.tag_count))
    io.write(string.format("Subtask count: %d\n", config.subtask_count))
    io.write(string.format("Description length: %d chars\n", config.description_length))
    io.write(string.format("Estimated size: ~%d bytes\n", M.get_estimated_size()))
end

return M
