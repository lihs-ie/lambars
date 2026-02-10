-- Test IDs for benchmarking with version management
-- REQ-UPDATE-IDS-001: ID/version pair management for conflict handling

local M = {}

local ID_POOL_SIZE = tonumber(os.getenv("ID_POOL_SIZE")) or 10
local SEED = tonumber(os.getenv("SEED"))

-- Create independent RNG for ID generation to avoid affecting common.lua random_*
local id_rng_state = SEED or 42

local static_task_ids = {
    "019bd467-1f27-7850-83b1-4ba96a937f04",
    "019bd467-1f30-7852-b18e-f13ceab605bc",
    "019bd467-1f38-7232-8f89-93444cbc6d59",
    "019bd467-1f40-7382-a91b-8a24f1448450",
    "019bd467-1f49-7db3-94b0-b2bb7ddafcad",
    "019bd467-1f52-72e2-a9cd-cb4f0b61fdfe",
    "019bd467-1f5b-7e31-b014-d7d81b122e95",
    "019bd467-1f63-7a00-be7d-bff9b95ec0d4",
    "019bd467-1f6c-7391-9589-3e7c9e6c337e",
    "019bd467-1f74-72d1-8e84-11be99b4c38a",
}

local function bit_rshift(a, n)
    return math.floor(a / (2 ^ n))
end

-- Simple LCG (Linear Congruential Generator) for reproducible ID generation
-- Does not affect global math.random() state
local function next_id_random()
    id_rng_state = (id_rng_state * 1103515245 + 12345) % 0x100000000
    return id_rng_state
end

local function generate_task_id(index)
    if index <= #static_task_ids then return static_task_ids[index] end

    -- Use independent RNG for reproducibility without affecting common.lua random_*
    local rand_base = next_id_random()
    local seq = index - #static_task_ids
    local part1 = (rand_base + seq * 0x12345) % 0x100000000
    local part2 = (rand_base + seq * 0x67) % 0x10000
    local part3 = 0x7000 + (bit_rshift(seq, 8) % 0x1000)
    local part4 = 0x8000 + (seq % 0x4000)
    local part5 = (index * 0x123456 + rand_base) % 0x1000000000000

    return string.format("%08x-%04x-%04x-%04x-%012x", part1, part2, part3, part4, part5)
end

local task_states = {}
for i = 1, ID_POOL_SIZE do
    table.insert(task_states, { id = generate_task_id(i), version = 1, status = "pending" })
end

local project_ids = {
    "019bd467-1f7c-7500-a25f-5d660c7b3710",
    "019bd467-1f85-7710-a756-d7e66b3ee53b",
    "019bd467-1f8d-7cb0-821f-f76bbbb254ca",
}

local function normalize_index(index, count)
    if type(index) ~= "number" then return nil, "index must be a number, got " .. type(index) end
    if index ~= math.floor(index) then return nil, "index must be an integer, got " .. tostring(index) end
    if count == 0 then return nil, "cannot index into empty array" end
    return ((index - 1) % count) + 1, nil
end

local function validate_version(version)
    if type(version) ~= "number" then return false, "version must be a number, got " .. type(version) end
    if version ~= math.floor(version) then return false, "version must be an integer, got " .. tostring(version) end
    if version < 1 then return false, "version must be >= 1, got " .. tostring(version) end
    return true, nil
end

function M.get_task_id(index)
    local actual_index, err = normalize_index(index, #task_states)
    if err then return nil, err end
    return task_states[actual_index].id, nil
end

function M.get_project_id(index)
    local actual_index, err = normalize_index(index, #project_ids)
    if err then return nil, err end
    return project_ids[actual_index], nil
end

function M.get_task_state(index)
    local actual_index, err = normalize_index(index, #task_states)
    if err then return nil, err end
    local state = task_states[actual_index]
    return { id = state.id, version = state.version, status = state.status }, nil
end

function M.increment_version(index)
    local actual_index, err = normalize_index(index, #task_states)
    if err then return nil, err end
    task_states[actual_index].version = task_states[actual_index].version + 1
    return task_states[actual_index].version, nil
end

function M.set_version(index, version)
    local actual_index, index_err = normalize_index(index, #task_states)
    if index_err then return nil, index_err end
    local valid, version_err = validate_version(version)
    if not valid then return nil, version_err end
    task_states[actual_index].version = version
    return true, nil
end

function M.set_version_and_status(index, version, status)
    local actual_index, index_err = normalize_index(index, #task_states)
    if index_err then return nil, index_err end
    local valid, version_err = validate_version(version)
    if not valid then return nil, version_err end
    task_states[actual_index].version = version
    task_states[actual_index].status = status
    return true, nil
end

function M.reset_versions()
    for i = 1, #task_states do
        task_states[i].version = 1
        task_states[i].status = "pending"
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
    for i, state in ipairs(task_states) do copy[i] = state.id end
    return copy
end

function M.get_all_project_ids()
    local copy = {}
    for i, id in ipairs(project_ids) do copy[i] = id end
    return copy
end

return M
