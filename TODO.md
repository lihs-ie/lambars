# TODO - Tasks Bulk 永続構造最適化

## In Progress

- [REFACTOR] RUST-005, RUST-006, RUST-007: NodePool 実装 - リファクタリング完了
  - Started: 2026-02-04
  - Goal: NodePool 構造体と関連メソッドの実装

## Next

- [ ] RUST-008: SearchIndexDelta ソート最適化
- [ ] BENCH-001: SearchIndexConfig への use_apply_bulk フラグ追加
- [ ] BENCH-002: save_tasks_bulk_optimized の apply_bulk 対応

## Done

- [x] RUST-001: BulkInsertResult<V> 構造体の追加 (2026-02-04)
- [x] RUST-002: insert_bulk_with_metrics メソッドの実装 (2026-02-04)
- [x] RUST-003: SearchIndexError enum の追加 (2026-02-04)
- [x] RUST-004: SearchIndex::apply_bulk メソッドの実装 (2026-02-04)
