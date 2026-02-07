-- Advanced Optics (Traversal, At, Filtered) benchmarks
-- benches/api/benchmarks/scripts/optics.lua
--
-- Endpoints:
--   PUT  /tasks/batch-update-field
--   PUT  /tasks/{id}/update-optional
--   PUT  /projects/{id}/metadata/{key}
--   PUT  /tasks/update-filtered
--   GET  /tasks/nested-access

package.path = package.path .. ";scripts/?.lua"
local common = require("common")
local test_ids = common.load_test_ids()

local counter = 0
local request_types = {"batch_update", "optional", "metadata", "filtered", "nested"}

-- Metadata keys for testing
local metadata_keys = {"owner", "deadline", "category", "version"}

function request()
    counter = counter + 1
    local req_type = request_types[(counter % #request_types) + 1]
    local task_id = test_ids.get_task_id(counter)
    local project_id = test_ids.get_project_id(counter)

    if req_type == "batch_update" then
        -- PUT /tasks/batch-update-field
        local fields = {"priority", "status"}
        local field = fields[(counter % #fields) + 1]
        local value = (field == "priority") and "high" or "in_progress"
        local body = common.json_encode({
            field = field,
            value = value,
            filter = {status = "pending"}
        })
        return wrk.format("PUT", "/tasks/batch-update-field", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "optional" then
        -- PUT /tasks/{id}/update-optional
        local actions = {"set", "clear", "modify"}
        local action = actions[(counter % #actions) + 1]
        local body = common.json_encode({
            field = "description",
            action = action,
            value = (action == "set") and "Updated description" or nil
        })
        return wrk.format("PUT", "/tasks/" .. task_id .. "/update-optional", {["Content-Type"] = "application/json"}, body)

    elseif req_type == "metadata" then
        -- PUT /projects/{id}/metadata/{key}
        local key = metadata_keys[(counter % #metadata_keys) + 1]
        local body = common.json_encode({
            value = "benchmark-value-" .. counter
        })
        return wrk.format("PUT", "/projects/" .. project_id .. "/metadata/" .. key, {["Content-Type"] = "application/json"}, body)

    elseif req_type == "filtered" then
        -- PUT /tasks/update-filtered
        local body = common.json_encode({
            filter = {
                priority = "high",
                status = "pending"
            },
            update = {
                status = "in_progress",
                add_tag = "processed"
            }
        })
        return wrk.format("PUT", "/tasks/update-filtered", {["Content-Type"] = "application/json"}, body)

    else
        -- GET /tasks/nested-access
        -- Query parameter is access_path, not path
        local paths = {"tasks.0.title", "tasks.0.subtasks.0.title", "projects.0.metadata.owner"}
        local path = paths[(counter % #paths) + 1]
        return wrk.format("GET", "/tasks/nested-access?access_path=" .. path)
    end
end

local handlers = common.create_standard_handlers("optics", {scenario_name = "optics", output_format = "json"})
function setup(thread) handlers.setup(thread) end
response = handlers.response
done = handlers.done
