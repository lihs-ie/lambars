# Phase 3: BULK_THRESHOLD 調整と Add-only 自動フォールバック

## In Progress

## Next
- [ ] ベンチマーク実行と RPS 確認

## Done
- [✅] RED: Phase 3 検証用テスト追加 (2026-02-06)
  - config_bulk_threshold_default_is_10_for_phase3
  - config_use_apply_bulk_default_is_true_for_phase3
  - config_builder_preserves_phase3_defaults

- [✅] GREEN: デフォルト値変更と既存バグ修正 (2026-02-06)
  - bulk_threshold: 100 → 10
  - use_apply_bulk: false → true
  - SearchIndexBulkBuilder::build の重複排除ロジック修正
    - 同じ token に対する異なる TaskId を全て保持するように修正
    - (token, task_id) の完全一致ペアのみ重複排除
  - 既存テストの期待値更新
    - config_default_optimization_flags
    - config_builder_default
    - config_use_apply_bulk_can_be_disabled (旧: config_use_apply_bulk_default_is_false)
    - test_builder_same_token_different_task_ids_all_preserved (旧: test_builder_duplicate_keys_last_wins)
    - test_builder_duplicate_token_task_id_pair_deduplicated (新規追加)

- [✅] REFACTOR: clippy/fmt チェック完了 (2026-02-06)
  - ドキュメントコメントのバッククォート追加
  - コードフォーマット適用
