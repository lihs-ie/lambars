-- Auto-generated test IDs for benchmarking
-- Generated at: 2026年 1月19日 月曜日 12時58分06秒 JST
-- API URL: http://localhost:3000

local M = {}

M.task_ids = {
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

M.project_ids = {
    "019bd467-1f7c-7500-a25f-5d660c7b3710",
    "019bd467-1f85-7710-a756-d7e66b3ee53b",
    "019bd467-1f8d-7cb0-821f-f76bbbb254ca",
}

-- Helper to get a task ID by index (with wrap-around)
function M.get_task_id(index)
    return M.task_ids[((index - 1) % #M.task_ids) + 1]
end

-- Helper to get a project ID by index (with wrap-around)
function M.get_project_id(index)
    return M.project_ids[((index - 1) % #M.project_ids) + 1]
end

return M
