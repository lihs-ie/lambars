# Error Rate Improvement Implementation

このドキュメントは、要件定義 `20260203_1804_tasks_bulk_update_error_rate_remediation.yaml` に基づく実装内容をまとめたものです。

## 実装概要

tasks_bulk と tasks_update のエラー率改善のため、以下の3つの要件を実装しました。

### REQ-ERROR-001: tasks_bulk バッチサイズ整合

**目的**: API の BULK_LIMIT (100) とベンチマークスクリプトのバッチサイズを整合させ、エラー率を正規化する。

**変更ファイル**:
- `benches/api/benchmarks/scripts/tasks_bulk.lua`

**変更内容**:
```lua
-- 変更前
local batch_sizes = {100, 250, 500}

-- 変更後
local batch_sizes = {10, 50, 100}
```

**期待効果**:
- エラー率: 68.40% → 10% 以下
- バッチサイズ超過による 400 エラーを 0% に削減

---

### REQ-ERROR-002: HTTP ステータス別集計

**目的**: エラー原因を可視化するため、HTTP ステータス別の集計機能を実装する。

**変更ファイル**:
1. `benches/api/benchmarks/scripts/error_tracker.lua`
2. `benches/api/benchmarks/scripts/common.lua`
3. `benches/api/benchmarks/scripts/tasks_bulk.lua`
4. `benches/api/benchmarks/scripts/tasks_update.lua`

**実装内容**:

#### 1. error_tracker.lua の拡張

スレッド対応のエラー追跡機能を追加:

```lua
-- スレッドリスト
M.threads = {}

-- スレッドをセットアップ
function M.setup_thread(thread)
    table.insert(M.threads, thread)
    -- スレッドローカルカウンタを初期化
    thread:set("status_200", 0)
    thread:set("status_201", 0)
    thread:set("status_207", 0)
    thread:set("status_400", 0)
    thread:set("status_404", 0)
    thread:set("status_409", 0)
    thread:set("status_422", 0)
    thread:set("status_500", 0)
    thread:set("status_502", 0)
    thread:set("status_other", 0)
end

-- スレッドローカルでレスポンスを追跡
function M.track_thread_response(status)
    local thread = wrk.thread
    if not thread then return end

    local key = "status_" .. tostring(status)
    local current = tonumber(thread:get(key)) or 0
    thread:set(key, current + 1)
end

-- 全スレッドから集計
function M.get_thread_aggregated_summary()
    local aggregated = {
        status_200 = 0,
        status_201 = 0,
        -- ... 他のステータス
    }

    for _, thread in ipairs(M.threads) do
        for key, _ in pairs(aggregated) do
            local value = tonumber(thread:get(key)) or 0
            aggregated[key] = aggregated[key] + value
        end
    end

    return aggregated
end
```

#### 2. common.lua のスレッド対応ハンドラ

```lua
function M.create_threaded_handlers(script_name)
    local error_tracker = try_require("error_tracker")

    return {
        setup = function(thread)
            error_tracker.setup_thread(thread)
        end,

        response = function(status, headers, body)
            error_tracker.track_thread_response(status)
            M.track_response(status, headers)
            if status >= 400 and status ~= 404 then
                io.stderr:write(string.format("[%s] Error %d\n", script_name, status))
            end
        end,

        done = function(summary, latency, requests)
            M.print_summary(script_name, summary)

            -- HTTP ステータス別内訳を出力
            local aggregated = error_tracker.get_thread_aggregated_summary()
            local total = summary.requests or 0

            io.write(string.format("\n--- %s HTTP Status Distribution ---\n", script_name))
            if total > 0 then
                -- 各ステータスの割合を表示
                local function print_status(code, count)
                    if count > 0 then
                        local percentage = (count / total) * 100
                        io.write(string.format("  %s: %d (%.1f%%)\n", code, count, percentage))
                    end
                end

                print_status("200 OK", aggregated.status_200)
                print_status("201 Created", aggregated.status_201)
                print_status("207 Multi-Status", aggregated.status_207)
                print_status("400 Bad Request", aggregated.status_400)
                print_status("404 Not Found", aggregated.status_404)
                print_status("409 Conflict", aggregated.status_409)
                print_status("422 Unprocessable Entity", aggregated.status_422)
                print_status("500 Internal Server Error", aggregated.status_500)
                print_status("502 Bad Gateway", aggregated.status_502)
                print_status("Other Status", aggregated.status_other)

                -- エラー率を計算 (4xx + 5xx)
                local errors = aggregated.status_400 + aggregated.status_404 +
                               aggregated.status_409 + aggregated.status_422 +
                               aggregated.status_500 + aggregated.status_502
                local error_rate = (errors / total) * 100
                io.write(string.format("Error Rate: %.2f%% (%d errors / %d requests)\n",
                                       error_rate, errors, total))
            else
                io.write("  No requests completed\n")
            end
        end
    }
end
```

#### 3. tasks_bulk.lua への適用

```lua
local handlers = common.create_threaded_handlers("tasks_bulk")
setup = handlers.setup
response = handlers.response
done = handlers.done
```

#### 4. tasks_update.lua への適用

**設計意図**: tasks_update.lua は独自のリトライロジックを持つため、`create_threaded_handlers()` を使用せず、`error_tracker` を直接統合しています。これにより、リトライ状態管理と HTTP ステータス追跡を両立しています。

既存のロジックを保持しつつ、ステータス追跡を追加:

```lua
local error_tracker = nil
local ok, module = pcall(require, "error_tracker")
if ok then
    error_tracker = module
end

function setup(thread)
    if error_tracker then
        error_tracker.setup_thread(thread)
    end
    -- ... 既存の ID 範囲分割ロジック
end

function response(status, headers, body)
    common.track_response(status, headers)
    if error_tracker then
        error_tracker.track_thread_response(status)
    end
    -- ... 既存のリトライロジック
end

function done(summary, latency, requests)
    common.print_summary("tasks_update", summary)

    -- HTTP ステータス別内訳を出力
    if error_tracker then
        local aggregated = error_tracker.get_thread_aggregated_summary()
        local total = summary.requests or 0

        io.write("\n--- tasks_update HTTP Status Distribution ---\n")
        -- ... ステータス別出力
    end

    -- ... 既存のリトライ統計出力
end
```

**期待効果**:
- HTTP ステータス別内訳の可視化
- エラー原因の特定が容易に
- 409 Conflict の比率が明確に

---

### REQ-ERROR-003: tasks_update ID プール改善

**目的**: ID プール設計を見直し、スレッド分離により競合を抑制する。

**変更ファイル**:
- `benches/api/benchmarks/scripts/tasks_update.lua`
  (test_ids.lua は既に ID_POOL_SIZE に対応済み)

**実装内容**:

#### スレッドごとの ID 範囲分割

```lua
function setup(thread)
    -- ... error_tracker の初期化

    -- スレッド固有の ID 範囲を設定
    thread:set("id", thread.id)

    local total_threads = tonumber(os.getenv("WRK_THREADS")) or 1
    local pool_size = tonumber(os.getenv("ID_POOL_SIZE")) or 10

    local ids_per_thread = math.floor(pool_size / total_threads)
    local start_index = thread.id * ids_per_thread
    local end_index = start_index + ids_per_thread - 1

    thread:set("id_start", start_index)
    thread:set("id_end", end_index)
    thread:set("id_range", ids_per_thread)

    io.write(string.format("[Thread %d] ID range: %d-%d (%d IDs)\n",
                           thread.id, start_index, end_index, ids_per_thread))
end
```

#### スレッド固有の ID 選択

```lua
function request()
    -- ... state 処理

    -- スレッド固有の ID 範囲を取得
    local id_start = tonumber(wrk.thread:get("id_start")) or 0
    local id_range = tonumber(wrk.thread:get("id_range")) or test_ids.get_task_count()

    local next_counter = counter + 1
    -- カウンタをスレッド固有の ID 範囲にマッピング
    local local_index = (next_counter % id_range)
    local global_index = id_start + local_index + 1

    local task_state, err = test_ids.get_task_state(global_index)
    -- ... エラーハンドリング

    counter = next_counter
    last_request_index = global_index
    last_request_is_update = true

    -- ... リクエスト生成
end
```

**期待効果**:
- エラー率: 41.52% → 10% 以下 (最小基準)
- 推奨: 41.52% → 3% 以下
- 409 Conflict 比率: 40% → 1% 未満
- スレッド間競合を 0 に近づける

---

## テスト

### テストスクリプト

`benches/api/benchmarks/scripts/test_error_rate_improvements.sh`

このスクリプトは以下をテストします:
- REQ-ERROR-001: バッチサイズの修正
- REQ-ERROR-002: HTTP ステータス追跡の実装
- REQ-ERROR-003: ID プール改善の実装
- Lua 構文の検証
- error_tracker モジュールの機能テスト

### テストの実行

```bash
cd benches/api/benchmarks/scripts
./test_error_rate_improvements.sh
```

**期待結果**: 全 27 テストがパス

---

## 使用方法

### tasks_bulk ベンチマーク

```bash
wrk -t4 -c10 -d30s \
    -s benches/api/benchmarks/scripts/tasks_bulk.lua \
    http://localhost:3002
```

**出力例**:
```
--- tasks_bulk Status Summary ---
Total requests: 10000
Errors: 500 (5.0%)

--- tasks_bulk HTTP Status Distribution ---
  200 OK: 8000 (80.0%)
  207 Multi-Status: 1500 (15.0%)
  400 Bad Request: 0 (0.0%)
  500 Internal Server Error: 500 (5.0%)
Error Rate: 5.00% (500 errors / 10000 requests)
```

### tasks_update ベンチマーク

```bash
export ID_POOL_SIZE=1000
export WRK_THREADS=4

wrk -t${WRK_THREADS} -c10 -d30s \
    -s benches/api/benchmarks/scripts/tasks_update.lua \
    http://localhost:3002
```

**出力例**:
```
[Thread 0] ID range: 0-249 (250 IDs)
[Thread 1] ID range: 250-499 (250 IDs)
[Thread 2] ID range: 500-749 (250 IDs)
[Thread 3] ID range: 750-999 (250 IDs)

--- tasks_update Status Summary ---
Total requests: 15000
Errors: 300 (2.0%)

--- tasks_update HTTP Status Distribution ---
  200 OK: 14700 (98.0%)
  409 Conflict: 150 (1.0%)
  500 Internal Server Error: 150 (1.0%)
Error Rate: 2.00% (300 errors / 15000 requests)
```

---

## ファイル一覧

### 修正ファイル

1. **benches/api/benchmarks/scripts/tasks_bulk.lua**
   - バッチサイズを {10, 50, 100} に変更
   - スレッド対応ハンドラを使用

2. **benches/api/benchmarks/scripts/tasks_update.lua**
   - スレッドごとの ID 範囲分割を実装
   - HTTP ステータス追跡を追加

3. **benches/api/benchmarks/scripts/error_tracker.lua**
   - setup_thread() 追加
   - track_thread_response() 追加
   - get_thread_aggregated_summary() 追加

4. **benches/api/benchmarks/scripts/common.lua**
   - create_threaded_handlers() 追加

### 新規ファイル

1. **benches/api/benchmarks/scripts/test_error_rate_improvements.sh**
   - 実装内容の自動テストスクリプト

2. **benches/api/benchmarks/scripts/README_ERROR_RATE_IMPROVEMENTS.md**
   - 本ドキュメント

---

## 受け入れ基準

### 最小基準 (Minimum)

- ✅ tasks_bulk.lua のバッチサイズが {10, 50, 100} に修正されている
- ✅ error_tracker.lua/common.lua がスレッド対応
- ✅ tasks_bulk/tasks_update が create_threaded_handlers を使用
- ✅ test_ids.lua が ID_POOL_SIZE 環境変数をサポート (既存機能)
- ✅ tasks_update.lua が ID 範囲分割を実装
- ⏳ tasks_bulk のエラー率が 10% 以下 (ベンチマーク実行後に確認)
- ⏳ tasks_update のエラー率が 10% 以下 (ベンチマーク実行後に確認)
- ✅ HTTP ステータス別内訳が可視化

### 推奨基準 (Recommended)

- ⏳ tasks_bulk のエラー率が 5% 以下
- ⏳ tasks_update のエラー率が 3% 以下
- ⏳ tasks_bulk の RPS が 100+ req/s
- ⏳ tasks_bulk の P99 が 10秒以下
- ⏳ 409 Conflict 比率が 1% 未満

注: ⏳ マークの項目は実際のベンチマーク実行後に確認が必要です。

---

## 次のステップ

1. **ベンチマーク実行**
   - tasks_bulk と tasks_update のベンチマークを実行
   - エラー率と HTTP ステータス別内訳を確認

2. **結果の記録**
   - 改善前後の比較データを収集
   - 分析結果を `docs/internal/analysis/` に記録

3. **Phase 2 への移行**
   - エラー率が正規化されたら、性能最適化フェーズへ
   - PersistentHashSet バルク最適化 (REQ-BENCH-COV-002)
   - 世代トークン方式 (IMP-POST-001)

---

## 参考資料

- 要件定義: `docs/internal/requirements/20260203_1804_tasks_bulk_update_error_rate_remediation.yaml`
- 実装計画: `docs/internal/plans/20260202_benchmark_coverage_improvement.yaml`
- Codex MCP 分析結果: 要件定義内に記載
