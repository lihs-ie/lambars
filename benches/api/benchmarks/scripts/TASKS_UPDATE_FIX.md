# tasks_update.lua 修正ドキュメント

## 問題の概要

CI で tasks_update.lua が以下のエラーで失敗していました:

```
PANIC: unprotected error in call to Lua API (.../lambars/benches/api/benchmarks/scripts/tasks_update.lua:46: attempt to perform arithmetic on field 'id' (a string value))
```

## 根本原因

1. **thread.id の型エラー**: `setup(thread)` 関数内で `thread.id` を直接算術演算に使用していましたが、`thread.id` は数値ではなく、文字列または nil の可能性がありました。
2. **wrk グローバル変数への安全でないアクセス**: `request()` 関数内で `wrk.format()` を直接呼び出していましたが、wrk2 実行時以外では `wrk` が未定義の可能性がありました。

## 修正内容

### 1. thread.id の数値変換

**Before:**
```lua
local start_index = thread.id * ids_per_thread
```

**After:**
```lua
local thread_id = tonumber(thread.id) or 0
local start_index = thread_id * ids_per_thread
```

**説明**: `thread.id` を `tonumber()` で数値に変換し、変換失敗時は 0 をデフォルト値として使用します。

### 2. wrk.format の安全なアクセス

**Before:**
```lua
return wrk.format("GET", "/health")
```

**After:**
```lua
if wrk and wrk.format then
    return wrk.format("GET", "/health")
else
    return ""
end
```

**説明**: `wrk` および `wrk.format` の存在を確認してから呼び出します。wrk2 実行時以外では空文字列を返します。

### 3. response() 関数の status nil チェック

**Before:**
```lua
function response(status, headers, body)
    common.track_response(status, headers)
```

**After:**
```lua
function response(status, headers, body)
    if not status then return end
    common.track_response(status, headers)
```

**説明**: `status` が nil の場合は早期リターンします。

## 修正箇所

- `benches/api/benchmarks/scripts/tasks_update.lua`
  - Line 46: `thread.id` を `tonumber()` で変換
  - Line 96, 103, 115, 123, 147, 160: `wrk.format()` の安全なアクセス
  - Line 134: `wrk.thread` の安全なアクセス
  - Line 167: `response()` 関数の `status` nil チェック

## 検証方法

### 1. Lua構文チェック

```bash
cd benches/api/benchmarks/scripts
lua5.1 -e "package.path = package.path .. ';scripts/?.lua'; dofile('tasks_update.lua')"
```

### 2. wrk2 による実行テスト

```bash
cd benches/api/benchmarks
./test_lua_compatibility.sh --target http://localhost:3002 --duration 3 --rate 5
```

### 3. CI での検証

GitHub Actions の API Workload Benchmark ワークフローで自動的に検証されます。

## 期待される結果

- wrk2 が tasks_update.lua を正常にロードできる
- `thread.id` の算術演算が動作する
- `wrk.format()` の呼び出しが安全に行われる
- CI のテストが PASSED になる

## 関連ファイル

- `benches/api/benchmarks/scripts/tasks_update.lua`: 修正対象
- `benches/api/benchmarks/scripts/test_tasks_update_fix.sh`: 修正検証スクリプト
- `benches/api/benchmarks/test_lua_compatibility.sh`: CI テストスクリプト

## 参考情報

- wrk2 の Lua API: https://github.com/giltene/wrk2
- wrk2 の thread オブジェクト仕様: スレッド ID は 0 から始まる整数だが、Lua では型変換が必要
- error_tracker.lua の実装: `wrk.thread` の nil チェックを参照
