# REQ-MEASURE-401: tasks_update 計測精度の修正

## 概要

tasks_update ベンチマークスクリプトにおいて、backoff/suppressed/fallback が early return で `track_response` を通過しないため、約40%のリクエストが未記録となっている問題を解決する。また、error_rate が部分母数で算出されているため、メトリクスを分離して正確な計測を実現する。

## 要件

### 機能要件

- backoff/suppressed/fallback リクエストを明示的にカウントし、excluded_requests として記録
- tracked_requests（実際にHTTPステータスが記録されたリクエスト）を明示化
- メトリクスを分離:
  - `success_rate`: 2xx / tracked_requests
  - `conflict_rate`: 409 / tracked_requests
  - `error_rate`: (4xx除く409 + 5xx) / tracked_requests
  - `server_error_rate`: 5xx / tracked_requests
- 整合性検証: `total_requests == sum(categories)` の検証ロジック追加

### 非機能要件

- 関数型プログラミング原則の遵守:
  - 純粋関数: 集計ロジックは副作用なし
  - 不変性: 計測データは不変
  - 副作用の分離: 副作用は呼び出し側に限定
- 既存の wrk スレッドモデルとの整合性を維持

## 使用方法

```bash
# tasks_update ベンチマークの実行
cd /Users/lihs/workspace/lambars
./benches/api/benchmarks/scripts/test_error_rate_accuracy.sh
```

## 技術スタック

- Lua 5.1 (wrk 互換)
- wrk HTTP benchmarking tool
- 既存モジュール: common.lua, error_tracker.lua

## 想定ユーザー

Lambars プロジェクトの開発者（ベンチマーク計測の精度を向上させる）
