# API 負荷プロファイル未反映問題の検証結果

## 概要

シナリオ YAML で定義された `target_rps` / `rps_profile` が実際の負荷生成に反映されていない問題の検証結果を記録する。

---

## RPS-REQ-001: レート制御欠落の確認

### 検査日時

2026-01-27

### 検査対象 1: run_benchmark.sh

- **ファイル**: `benches/api/benchmarks/run_benchmark.sh`
- **行番号**: 1464-1467

### 検査結果 (run_benchmark.sh)

```bash
if wrk -t"${THREADS}" -c"${CONNECTIONS}" -d"${DURATION}" \
    --latency \
    --script="scripts/${script_name}.lua" \
    "${API_URL}" 2>&1 | tee "${result_file}"; then
```

### 検査対象 2: load_profile.lua

- **ファイル**: `benches/api/benchmarks/scripts/load_profile.lua`
- **行番号**: 188-191

### 検査結果 (load_profile.lua)

```lua
-- Simple rate limiting using delay calculation
-- NOTE: wrk does not support true rate limiting in request(),
-- so this returns a calculated delay. Use with wrk's --rate option for actual limiting.
```

### 結論

- `--rate` オプションが未使用
- `load_profile.lua` 自体も wrk の制限を認識している（コメントで明記）
- wrk は open-loop レート制御をサポートしておらず、クローズドループ負荷（レスポンス時間依存）になっている

**補足**: wrk (https://github.com/wg/wrk) は `--rate` オプションをサポートしていない。open-loop レート制御には wrk2 (https://github.com/giltene/wrk2) の `-R` オプションが必要。

---

## RPS-REQ-002 追加検証: 環境変数名の不整合

### 検査対象 1: run_benchmark.sh のエクスポート

- **ファイル**: `benches/api/benchmarks/run_benchmark.sh`
- **行番号**: 360-369

### 検査結果 (run_benchmark.sh)

```bash
# RPS profile: constant -> steady, ramp_up_down -> ramp, burst -> burst
local rps_profile
rps_profile=$(yq '.rps_profile // "constant"' "${scenario_file}" | tr -d '"')
case "${rps_profile}" in
    "constant")    export RPS_PROFILE="steady" ;;
    "ramp_up_down") export RPS_PROFILE="ramp" ;;
    "burst")       export RPS_PROFILE="burst" ;;
    "step_up")     export RPS_PROFILE="steady" ;;  # ← 問題: step_up が steady にマップされている
    *)             export RPS_PROFILE="steady" ;;
esac
```

**確認**: `export LOAD_PROFILE` は run_benchmark.sh に**存在しない**（grep で確認済み）。

### 検査対象 2: Lua スクリプトの環境変数参照

- **ファイル 1**: `benches/api/benchmarks/scripts/profile_wrk.lua:43`
- **ファイル 2**: `benches/api/benchmarks/scripts/load_shape_demo.lua:61`

### 検査結果 (Lua スクリプト)

```lua
-- profile_wrk.lua:43
load_profile = os.getenv("LOAD_PROFILE") or "constant",

-- load_shape_demo.lua:61
args.profile = os.getenv("LOAD_PROFILE") or args.profile
```

### 問題点

1. **環境変数名の不整合**: run_benchmark.sh は `RPS_PROFILE` をエクスポートするが、Lua スクリプトは `LOAD_PROFILE` を参照している。そのため、Lua スクリプトは常にデフォルト値 `"constant"` を使用する。
2. **step_up が steady にマップされている**: `step_up` プロファイルを持つシナリオ（例: `step_up_stress_test.yaml`）を実行しても、内部的には `steady` として扱われ、フェーズ分割が動作しない。
3. **ramp_up_down が ramp にマップされている**: `load_profile.lua` は `ramp_up_down` を期待するが、`ramp` にマップされており不整合が生じる。
4. **constant が steady にマップされている**: 同様に `load_profile.lua` は `constant` を期待する。

### 結論

1. **環境変数名の不整合により、Lua スクリプトはシナリオ YAML の rps_profile を受け取れない**
2. rps_profile のマッピング値も `load_profile.lua` の定義と一致しておらず、仮に環境変数名を修正しても不整合が残る

---

## RPS-REQ-002: シナリオ反映不備の検証結果

### 検証シナリオ（シナリオ YAML からの実値）

| シナリオ | rps_profile | target_rps | ソース行 |
|----------|-------------|------------|----------|
| tasks_eff.yaml | constant | 1000 | 行31 |
| step_up_stress_test.yaml | step_up | 1000 | 行36 |
| ramp_up_down_test.yaml | ramp_up_down | 500 | 行35 |

### コードレベル検証による問題確認

ベンチマーク実行による実測値取得は不要である。以下のコードレベル検証により、問題が確実に存在することを確認した:

1. **`--rate` オプション未使用** (run_benchmark.sh:1464-1467)
   - wrk コマンドに `--rate` オプションがない
   - wrk は `--rate` をサポートしていない（wrk2 の `-R` が必要）
   - よって、`target_rps` が送信レートに反映されることは**不可能**

2. **環境変数名の不整合** (上記セクションで詳述)
   - run_benchmark.sh は `RPS_PROFILE` をエクスポート
   - Lua スクリプトは `LOAD_PROFILE` を参照
   - Lua スクリプトは常にデフォルト値 `"constant"` を使用

3. **rps_profile マッピングの問題** (run_benchmark.sh:360-369)
   - `step_up` → `steady` にマップされ、フェーズ分割が動作しない
   - `ramp_up_down` → `ramp` にマップされ、`load_profile.lua` との不整合が生じる
   - `constant` → `steady` にマップされ、同様に不整合が生じる

### 差分記録（コードレベル検証）

| シナリオ | rps_profile | 期待される LOAD_PROFILE | 実際の RPS_PROFILE | Lua スクリプトが受け取る値 | 問題 |
|----------|-------------|-------------------------|--------------------|-----------------------------|------|
| tasks_eff | constant | constant | steady | **constant (デフォルト)** | 環境変数名不整合 |
| step_up_stress_test | step_up | step_up | steady | **constant (デフォルト)** | フェーズ分割が動作しない + 環境変数名不整合 |
| ramp_up_down_test | ramp_up_down | ramp_up_down | ramp | **constant (デフォルト)** | 環境変数名不整合 |

### 結論

`target_rps` / `rps_profile` がレート制御に反映されていない。

1. **wrk は `--rate` オプションをサポートしていない**: シナリオで定義した `target_rps` は送信レートに反映されない
2. **環境変数名の不整合**: run_benchmark.sh は `RPS_PROFILE` をエクスポートするが、Lua スクリプトは `LOAD_PROFILE` を参照するため、プロファイル情報が伝達されない
3. **rps_profile のマッピングが不正確**: 特に `step_up` が `steady` にマップされており、仮に環境変数名を修正してもフェーズ分割が動作しない
4. **クローズドループ負荷**: レスポンス時間に依存した負荷生成になっており、再現性のある負荷制御ができない

---

## wrk2 vs k6 比較検討

### 比較表

| 観点 | wrk2 | k6 |
|------|------|-----|
| レート制御 | `-R` オプションで open-loop | stages/scenarios で精密制御 |
| Lua 互換性 | wrk と完全互換 | JavaScript/Lua 非互換 |
| 導入コスト | 低（Lua 資産再利用可） | 高（全スクリプト書き換え） |
| フェーズ分割 | スクリプト側で対応必要 | 組み込みサポート |
| 出力形式 | wrk 互換（既存パーサー使用可） | 独自形式（パーサー要対応） |

### 結論

既存 Lua 資産（25 スクリプト）を活かすため wrk2 を採用。k6 移行は将来の拡張として記録 (RPS-FUT-001)。

---

## 修正方針

1. wrk を wrk2 に置換し、`-R` オプションでレート制御を有効化
2. rps_profile に応じたフェーズ分割を実装（step_up, ramp_up_down, burst）
3. 実測 RPS と target_rps の乖離を ±5% 以内に収める
4. meta.json 互換性を維持しつつ、フェーズ統合結果を反映

---

## 関連要件

- RPS-REQ-001: レート制御欠落の確認 -> **完了**
  - 証拠: run_benchmark.sh:1464-1467 に `--rate` オプションなし
  - 証拠: load_profile.lua:188-191 に「wrk does not support true rate limiting」コメント
- RPS-REQ-002: シナリオ反映不備の検証 -> **完了**
  - 証拠: コードレベル検証により、`target_rps` が送信レートに反映されないことを確認
  - 証拠: rps_profile マッピングの問題（step_up -> steady）を確認
  - 証拠: 環境変数名の不整合（RPS_PROFILE vs LOAD_PROFILE）を確認
    - run_benchmark.sh:360-369 で `RPS_PROFILE` をエクスポート
    - profile_wrk.lua:43, load_shape_demo.lua:61 で `LOAD_PROFILE` を参照
    - run_benchmark.sh に `export LOAD_PROFILE` が存在しない（grep で確認）
