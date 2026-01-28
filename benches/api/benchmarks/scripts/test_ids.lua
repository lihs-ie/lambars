-- Auto-generated test IDs for benchmarking
-- Generated at: 2026年 1月19日 月曜日 12時58分06秒 JST
-- API URL: http://localhost:3000
--
-- REQ-UPDATE-IDS-001: ID と version をペアで管理する
-- - get_task_state(index) で ID/version ペアを取得
-- - increment_version(index) で更新成功時に version をインクリメント
-- - reset_versions() でベンチマーク開始時に全 version を 1 にリセット
--
-- wrk2 スレッドモデルに関する注意:
-- 各スレッドは独立した Lua 状態を持つため、同一 ID を複数スレッドで
-- 更新すると、Lua 側の version がスレッド間で分岐する。
-- 低衝突シナリオ（single writer）では問題ないが、高衝突シナリオでは
-- サーバ側の version との不整合が発生する可能性がある。
-- 高衝突シナリオでは、409 発生時に GET で最新 version を取得して再試行する。
--
-- 状態変更に関する注意:
-- increment_version / set_version / reset_versions は内部状態を破壊的に変更する。
-- これは wrk2 のスレッドモデルで各スレッドが独立した状態を持つ必要があるため。
-- 関数型プログラミングの純粋性は犠牲になるが、ベンチマークスクリプトの設計上必要。

local M = {}

-- Task states: ID と version をペアで管理（内部データ）
local task_states = {
    { id = "019bd467-1f27-7850-83b1-4ba96a937f04", version = 1 },
    { id = "019bd467-1f30-7852-b18e-f13ceab605bc", version = 1 },
    { id = "019bd467-1f38-7232-8f89-93444cbc6d59", version = 1 },
    { id = "019bd467-1f40-7382-a91b-8a24f1448450", version = 1 },
    { id = "019bd467-1f49-7db3-94b0-b2bb7ddafcad", version = 1 },
    { id = "019bd467-1f52-72e2-a9cd-cb4f0b61fdfe", version = 1 },
    { id = "019bd467-1f5b-7e31-b014-d7d81b122e95", version = 1 },
    { id = "019bd467-1f63-7a00-be7d-bff9b95ec0d4", version = 1 },
    { id = "019bd467-1f6c-7391-9589-3e7c9e6c337e", version = 1 },
    { id = "019bd467-1f74-72d1-8e84-11be99b4c38a", version = 1 },
}

local project_ids = {
    "019bd467-1f7c-7500-a25f-5d660c7b3710",
    "019bd467-1f85-7710-a756-d7e66b3ee53b",
    "019bd467-1f8d-7cb0-821f-f76bbbb254ca",
}

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
    -- wrap-around: ((index - 1) % count) + 1
    -- 負数や 0 も正しく wrap-around される
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

-- Helper to get a task ID by index (with wrap-around)
-- @param index 1-based index (wrap-around supported)
-- @return task ID string, or nil and error message
function M.get_task_id(index)
    local actual_index, err = normalize_index(index, #task_states)
    if err then
        return nil, err
    end
    return task_states[actual_index].id, nil
end

-- Helper to get a project ID by index (with wrap-around)
-- @param index 1-based index (wrap-around supported)
-- @return project ID string, or nil and error message
function M.get_project_id(index)
    local actual_index, err = normalize_index(index, #project_ids)
    if err then
        return nil, err
    end
    return project_ids[actual_index], nil
end

-- REQ-UPDATE-IDS-001: ID と version をペアで取得
-- @param index 1-based index (wrap-around supported)
-- @return { id = string, version = number }, or nil and error message
function M.get_task_state(index)
    local actual_index, err = normalize_index(index, #task_states)
    if err then
        return nil, err
    end
    local state = task_states[actual_index]
    return { id = state.id, version = state.version }, nil
end

-- REQ-UPDATE-IDS-001: 更新成功時に version をインクリメント
-- 注意: 内部状態を破壊的に変更する（wrk2 スレッドモデルのため）
-- @param index 1-based index (wrap-around supported)
-- @return new version number, or nil and error message
function M.increment_version(index)
    local actual_index, err = normalize_index(index, #task_states)
    if err then
        return nil, err
    end
    task_states[actual_index].version = task_states[actual_index].version + 1
    return task_states[actual_index].version, nil
end

-- REQ-UPDATE-IDS-001: 外部から取得した version で上書き
-- 注意: 内部状態を破壊的に変更する（wrk2 スレッドモデルのため）
-- @param index 1-based index (wrap-around supported)
-- @param version new version number (must be positive integer)
-- @return true on success, or nil and error message
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

-- REQ-UPDATE-IDS-001: 全 version を 1 にリセット（ベンチマーク開始時）
-- 注意: 内部状態を破壊的に変更する（wrk2 スレッドモデルのため）
function M.reset_versions()
    for i = 1, #task_states do
        task_states[i].version = 1
    end
end

-- 現在の task_states のサイズを取得
function M.get_task_count()
    return #task_states
end

-- 現在の project_ids のサイズを取得
function M.get_project_count()
    return #project_ids
end

-- task_ids のコピーを取得（後方互換性のため）
-- 注意: 内部データのコピーを返す。変更しても内部には影響しない。
function M.get_all_task_ids()
    local copy = {}
    for i, state in ipairs(task_states) do
        copy[i] = state.id
    end
    return copy
end

-- project_ids のコピーを取得（後方互換性のため）
-- 注意: 内部データのコピーを返す。変更しても内部には影響しない。
function M.get_all_project_ids()
    local copy = {}
    for i, id in ipairs(project_ids) do
        copy[i] = id
    end
    return copy
end

return M
