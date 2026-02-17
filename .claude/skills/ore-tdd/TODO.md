# TODO - profiling.yml ubuntu-22.04 pin

## In Progress

- [🔄 REFACTOR] pin profiling runners to ubuntu-22.04 - リファクタリング中
  - Started: 2026-02-17
  - Goal: テストスクリプトのリファクタリング（期待値定数化）

## Next

(なし)

## Done

- [x] AP-1: TaskId Copy 化 (IMPL-AH-001~004) (2026-02-12)
  - 332 clone_on_copy fixes across 17 files
- [x] AP-2: mimalloc 導入 (IMPL-AH-005~006c) (2026-02-12)
  - feature gate + Dockerfile + compose.ci.yaml
- [x] AP-3: fast-hash feature (IMPL-AH-007) (2026-02-12)
  - lambars/fxhash feature passthrough
- [x] AP-5: SearchIndexWriterConfig::for_bulk_workload (IMPL-AH-009~011) (2026-02-12)
  - const factory method + WRITER_PROFILE env var + 3 tests
