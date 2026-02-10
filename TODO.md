# TODO - Tasks Bulk/Tasks Update ボトルネック改善

## In Progress
- [RED] [IMPL-PRB1-002-001] CompactionBudget を SegmentOverlayConfig に導入 - 失敗テスト作成中
  - Started: 2026-02-11
  - Goal: budget 駆動のコンパクション制御を導入し、append のみ/compact_one/強制 compact を budget パラメータで切り替え可能にする

## Next
- [ ] [Phase 2-1] insert_bulk_owned の全経路適用
- [ ] [Phase 2-2] Builder パターンによる世代トークン管理
- [ ] [Phase 3-1] ChildSlot の SmallVec 化
- [ ] [Phase 3-2] ScratchBuffer による中間 Vec の再利用

## Done
- [x] [Phase 1] OrderedUniqueSet SortedVec 化 + merge/difference/intersection 実装 (2026-02-05)
  - Large 表現を PersistentHashSet から SortedVec (Arc<Vec<T>>) に変更
  - contains の Large 経路を二分探索に変更 (O(log n))
  - merge メソッドの実装（two-pointer アルゴリズム、O(n+m)）
  - difference メソッドの実装（線形時間、O(n+m)）
  - intersection メソッドの実装（線形時間、O(n+m)）
  - iter_sorted の Large 経路をソート不要に最適化 (O(n))

- [x] RUST-001: BulkInsertResult<V> 構造体の追加 (2026-02-04)
- [x] RUST-002: insert_bulk_with_metrics メソッドの実装 (2026-02-04)
- [x] RUST-003: SearchIndexError enum の追加 (2026-02-04)
- [x] RUST-004: SearchIndex::apply_bulk メソッドの実装 (2026-02-04)
- [x] RUST-005: NodePool 構造体の実装 (2026-02-04)
- [x] RUST-006: NodePoolMetrics 構造体の実装 (2026-02-04)
- [x] RUST-007: insert_bulk_with_pool メソッドの実装 (2026-02-04)
- [x] RUST-008: SearchIndexDelta ソート最適化 (2026-02-04)
- [x] BENCH-001: SearchIndexConfig への use_apply_bulk フラグ追加 (2026-02-04)
- [x] BENCH-002: apply_changes の apply_bulk 対応 (2026-02-04)
