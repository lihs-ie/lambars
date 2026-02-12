# IMPL-TBPA2-003: merge 経路選択率を meta に出力し CI ゲート化

## 概要

tasks_bulk の merge 経路選択率（with_arena vs without_arena）を meta.json に出力し、CI で自動検知できるゲートを追加する。今回の不具合は「with_arena 実装が存在しても経路に乗っていない」ことが原因であり、経路選択率を出さない限り再発を検知できない。

## 要件

### 背景

- Run 21886689088 の tasks_bulk は p99=8320ms で閾値 500ms を大幅超過
- stacks.folded の分析で `merge_index_delta_add_only_owned_with_arena` が 0.0037%、`merge_index_delta_add_only_owned` が 0.7716% で、with_arena 経路が使われていない
- bulk 経路が非 arena 実装に分岐していたため、CompactionBudget + MergeArena の効果が出なかった

### 変更内容

**注意**: Rust 側のカウンタ実装（query.rs）は別のサブエージェントが担当する予定。このタスクでは **Shell/Lua スクリプト側の対応のみ** を行う。

**実装完了**: IMPL-TBLR-002 と IMPL-TBLR-003 が完了し、merge_path_detail の fail ゲートと regression guard が実装された。

#### 1. `benches/api/benchmarks/run_benchmark.sh`

tasks_bulk の meta.json 生成時に `merge_path_detail` を追加する。Rust サーバのレスポンスヘッダまたはログから取得する仕組みの代わりに、**stacks.folded からプロファイルベースで計算**する方式を採用:

```bash
# stacks.folded から with_arena / without_arena の比率を計算
if [ -f "${RESULTS_DIR}/stacks.folded" ]; then
  total_samples=$(awk '{s+=$NF} END{print s}' "${RESULTS_DIR}/stacks.folded")
  with_arena=$(awk '$0~/merge_index_delta_add_only_owned_with_arena/{s+=$NF} END{print s+0}' "${RESULTS_DIR}/stacks.folded")
  without_arena=$(awk '$0~/merge_index_delta_add_only_owned[^_]/{s+=$NF} END{print s+0}' "${RESULTS_DIR}/stacks.folded")
  # merge_path_detail を meta.json に追記
fi
```

**merge_path_detail のスキーマ**:
```json
{
  "merge_path_detail": {
    "with_arena_samples": <number>,
    "without_arena_samples": <number>,
    "with_arena_ratio": <float 0-1>
  }
}
```

#### 2. `benches/api/benchmarks/check_thresholds.sh`

**IMPL-TBLR-002**: merge_path_detail 欠落を fail 条件に変更

- merge_path_detail が欠落している場合は **exit code 3 で fail**（WARNING から変更）
- bulk_with_arena/bulk_without_arena が不完全な場合も **exit code 3 で fail**
- これにより CI で確実に検知可能

**IMPL-TBLR-003**: tasks_bulk の回帰防止ゲートを段階化

- `check_bulk_regression_guard` 関数を追加
- P99 <= 9550ms, RPS >= 341.36 を revert 閾値として設定
- どちらか違反した場合は exit code 3 で fail
- Run 21886689088 のベースライン性能への回帰を防止

#### 3. テストスクリプト

`benches/api/benchmarks/scripts/test_merge_path_gate.sh` を更新:

- Test 1-8: 既存テスト（merge_path_detail の基本動作）
- Test 3: WARNING case → **FAIL case に変更**（merge_path_detail 欠落）
- Test 9: FAIL case（bulk_with_arena/bulk_without_arena が空文字列）
- Test 10: FAIL case（merge_path_error フィールドのみ存在）
- Test 11: PASS case（regression guard 正常）
- Test 12-14: FAIL case（regression guard 違反パターン）
- 全 14 テストが PASS することを確認

### 受入基準

- tasks_bulk の meta.json に `merge_path_detail` が出力される
- `with_arena_ratio` 未達時に check_thresholds が exit code 3 を返す
- テストスクリプトが全パスする
- コミットメッセージ: `feat(bench): add merge path telemetry and CI gate for tasks_bulk`

## 技術スタック

- Bash
- awk (プロファイル解析)
- jq (JSON 操作)

## 想定ユーザー

- lambars プロジェクトのベンチマークスクリプト保守担当者
- tasks_bulk の merge 経路選択率を監視する開発者
