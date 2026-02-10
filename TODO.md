# TODO - Tasks Bulk/Tasks Update ボトルネック改善

## In Progress
- [GREEN] [IMPL-PRB1-001-003] update_task に read-repair ループ追加 - 最小実装完了
  - Started: 2026-02-11
  - Goal: update_task に bounded read-repair CAS ループを追加し stale-version 409 を吸収

## Done
- [x] [IMPL-PRB1-001-001] ConflictKind 列挙型の導入 (2026-02-11)
- [ ] [Phase 2-1] insert_bulk_owned の全経路適用
- [ ] [Phase 2-2] Builder パターンによる世代トークン管理
- [ ] [Phase 3-1] ChildSlot の SmallVec 化
- [ ] [Phase 3-2] ScratchBuffer による中間 Vec の再利用

## Done
- [x] [IMPL-PRB1-002] CompactionBudget + MergeArena 導入 (2026-02-11)
  - CompactionBudget struct (soft/hard segment thresholds)
  - MergeArena struct (reusable scratch buffer with auto-shrink)
  - Budget-driven append_and_compact (3-tier: append only / compact_one / force compact)
  - Arena-backed merge_index_delta_add_only_owned_with_arena
  - Arena-backed apply_delta_owned_with_arena
  - writer_loop に MergeArena 配置 (バッチ間再利用)
  - apply_coalesced_changes に arena パラメータ追加

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
