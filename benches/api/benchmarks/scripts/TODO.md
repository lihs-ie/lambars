# TODO

## In Progress

- [🔴 RED] tasks_update.lua に request_categories を追加
  - Started: 2026-02-05T12:00:00+09:00
  - Goal: backoff/suppressed/fallback/executed の4カテゴリを追加し、各 early return でカウントする

## Next

- [ ] tasks_update.lua の done() でカテゴリ別集計を出力
- [ ] result_collector.lua に excluded_requests / tracked_requests を追加
- [ ] result_collector.lua でメトリクスを分離（success_rate / conflict_rate / error_rate / server_error_rate）
- [ ] 整合性検証ロジックを追加
- [ ] 既存のベンチマークスクリプトで動作確認

## Done

(完了済みタスクなし)
