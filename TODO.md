# TODO - Tasks Bulk 永続構造最適化

## In Progress

- [GREEN] RUST-008: SearchIndexDelta ソート最適化 - 実装完了
  - Started: 2026-02-04
  - Goal: バルク操作時のソートを最小化する

## Next

- [ ] BENCH-001: SearchIndexConfig への use_apply_bulk フラグ追加
- [ ] BENCH-002: save_tasks_bulk_optimized の apply_bulk 対応

## Done

- [x] RUST-001: BulkInsertResult<V> 構造体の追加 (2026-02-04)
- [x] RUST-002: insert_bulk_with_metrics メソッドの実装 (2026-02-04)
- [x] RUST-003: SearchIndexError enum の追加 (2026-02-04)
- [x] RUST-004: SearchIndex::apply_bulk メソッドの実装 (2026-02-04)
- [x] RUST-005: NodePool 構造体の実装 (2026-02-04)
- [x] RUST-006: NodePoolMetrics 構造体の実装 (2026-02-04)
- [x] RUST-007: insert_bulk_with_pool メソッドの実装 (2026-02-04)
