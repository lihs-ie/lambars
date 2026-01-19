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

-- Try to load generated test IDs
local test_ids
local ok, ids = pcall(require, "test_ids")
if ok then
    test_ids = ids
else
    test_ids = {
        task_ids = {
            "a1b2c3d4-e5f6-4789-abcd-ef0123456789",
            "b2c3d4e5-f6a7-4890-bcde-f01234567890",
            "c3d4e5f6-a7b8-4901-cdef-012345678901"
        },
        project_ids = {
            "f6a7b8c9-d0e1-4234-fabc-345678901234",
            "a7b8c9d0-e1f2-4345-abcd-456789012345"
        },
        get_task_id = function(index) return test_ids.task_ids[((index - 1) % #test_ids.task_ids) + 1] end,
        get_project_id = function(index) return test_ids.project_ids[((index - 1) % #test_ids.project_ids) + 1] end
    }
end

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

function response(status, headers, body)
    common.track_response(status)
    if status >= 400 and status ~= 404 then
        io.stderr:write(string.format("[optics] Error %d\n", status))
    end
end

function done(summary, latency, requests)
    common.print_summary("optics", summary)
end
