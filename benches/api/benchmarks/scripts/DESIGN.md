# DESIGN

## アーキテクチャ概要

REQ-MEASURE-401 の実装は以下の2つのファイルに分散する:

1. **tasks_update.lua**: リクエスト生成とカテゴリ別カウント
2. **result_collector.lua**: 集計とメトリクス算出

関数型プログラミングの原則に従い、純粋関数での集計ロジックを実装し、副作用は呼び出し側（done() 関数）に限定する。

## 設計判断

### リクエストカテゴリの分類

**決定**: リクエストを4つのカテゴリに分類
- `executed`: 実際にAPIに送信され、HTTPステータスが記録されたリクエスト
- `backoff`: Conflict後の再試行待機中に送信された /health リクエスト
- `suppressed`: スレッド数がID_POOL_SIZEを超えたため抑制されたリクエスト
- `fallback`: エラーハンドリングのフォールバックとして送信された /health リクエスト

**理由**:
- 約40%のリクエストが未記録となる問題を解決するため
- エラー率の分母を正確にするため（executed のみをベースとする）

**代替案**:
- すべてのリクエストを記録する（backoff/suppressed も track_response に含める）
  → 却下理由: error_rate の分母がさらに曖昧になる

### メトリクスの分離

**決定**: 以下のメトリクスを分離
- `success_rate`: 2xx / tracked_requests
- `conflict_rate`: 409 / tracked_requests
- `error_rate`: (4xx除く409 + 5xx) / tracked_requests
- `server_error_rate`: 5xx / tracked_requests

**理由**:
- 409 Conflict は楽観的ロックの仕様であり、エラーとは区別すべき
- 5xx はサーバー側の問題であり、4xx とは分離して分析すべき

**代替案**:
- error_rate に 409 を含める
  → 却下理由: Conflict はエラーではなく、正常な再試行フローの一部

### 整合性検証

**決定**: `total_requests == sum(categories)` を検証
- 不整合時は警告を出力

**理由**:
- 計測漏れを検出するため
- wrk のスレッドモデルに起因するカウント誤差を早期検出

**代替案**:
- 検証なし
  → 却下理由: 計測精度の保証が困難

## 技術的詳細

### データ構造

#### tasks_update.lua のグローバル変数

```lua
local request_categories = {
    executed = 0,        -- track_response を通過したリクエスト
    backoff = 0,         -- backoff 中の /health リクエスト
    suppressed = 0,      -- スレッド抑制による /health リクエスト
    fallback = 0         -- フォールバック /health リクエスト
}
```

#### result_collector.lua のメトリクス

```lua
M.results.meta = {
    excluded_requests = 0,      -- backoff + suppressed + fallback
    tracked_requests = 0,       -- sum(http_status)
    total_requests = 0          -- executed + excluded_requests
}
```

### インターフェース

#### tasks_update.lua

- `done(summary, latency, requests)`: カテゴリ別集計を出力

#### result_collector.lua

- `M.finalize(summary, latency, requests)`: メトリクス算出
- `M.format_json()`: JSON 形式で出力
- `M.format_yaml()`: YAML 形式で出力

### 純粋関数の分離

集計ロジックは純粋関数として実装:

```lua
-- 純粋関数: カテゴリ別集計
local function aggregate_categories()
    return {
        executed = request_categories.executed,
        backoff = request_categories.backoff,
        suppressed = request_categories.suppressed,
        fallback = request_categories.fallback
    }
end

-- 純粋関数: 整合性検証
local function verify_consistency(categories, total_requests)
    local sum = categories.executed + categories.backoff +
                categories.suppressed + categories.fallback
    return sum == total_requests, sum
end
```

副作用（出力）は呼び出し側に限定:

```lua
function done(summary, latency, requests)
    local categories = aggregate_categories()  -- 純粋関数
    local is_consistent, sum = verify_consistency(categories, summary.requests)  -- 純粋関数

    -- 副作用: 出力
    if not is_consistent then
        io.stderr:write(string.format(
            "[tasks_update] WARN: Inconsistency detected: total=%d, sum(categories)=%d\n",
            summary.requests, sum))
    end

    -- 副作用: メトリクス出力
    io.write(string.format("Request categories:\n"))
    for category, count in pairs(categories) do
        io.write(string.format("  %s: %d\n", category, count))
    end
end
```

## 制約事項

- wrk のスレッドモデルにより、スレッド間のカウント共有が困難
- error_tracker モジュールに依存（スレッド間集計）
- Lua 5.1 の機能制限（クロージャの制約など）

## 将来の拡張性

- カテゴリの追加が容易（request_categories に新しいキーを追加）
- メトリクスの追加が容易（aggregate_error_counts 関数を拡張）
- 整合性検証の強化（閾値ベースの警告など）
