# tasks_bulk スケール別ベンチマークガイド

## 概要

`tasks_bulk` エンドポイントのバッチサイズ別性能特性を測定するためのガイドです。小規模（10エントリ）から超大規模（100,000エントリ）まで、段階的にスケールアップしながら測定を行います。

## スケール一覧

| スケール | エントリ数 | 目的 | 優先度 | 実行条件 |
|---------|-----------|------|--------|---------|
| Small | 10 | 小規模バッチの baseline | 必須 | なし |
| Medium | 100 | 中規模バッチの性能特性 | 必須 | なし |
| Large | 1,000 | 大規模バッチの性能特性 | 必須 | なし |
| XLarge | 10,000 | 超大規模バッチの性能特性 | 条件付き | Large が成功 |
| XXLarge | 100,000 | 上限テスト | 条件付き | XLarge が成功 + 追加条件 |

## 測定項目

全てのスケールで以下の項目を測定します:

1. **RPS（Requests per Second）**
   - 1秒あたりのリクエスト処理数

2. **レイテンシ**
   - P50（中央値）
   - P95（95パーセンタイル）
   - P99（99パーセンタイル）

3. **総合エラー率**
   - 非2xxレスポンスの割合

4. **HTTPステータス別エラー率**
   - 2xx 成功率
   - 409 Conflict 率
   - 422 Validation Error 率
   - 5xx Server Error 率

5. **メモリ使用量（可能であれば）**
   - API サーバーのメモリ使用量

## 各スケールの詳細

### Small (10エントリ)

**目的**: 小規模バッチの baseline 測定。オーバーヘッドの把握。

**実行方法**:

```bash
cd benches/api/benchmarks
# 注: tasks_bulk_10.lua は今後実装予定
# 現時点では tasks_bulk.lua を修正して使用
# wrk2 を使用して固定レート(-R)で測定
wrk2 -t2 -c10 -d30s -R100 -s scripts/tasks_bulk.lua http://localhost:3002
```

**期待される結果**:
- RPS: 80-100 req/s（-R100 の固定レート以下）
- P50 レイテンシ: < 100ms
- P95 レイテンシ: < 300ms
- P99 レイテンシ: < 500ms
- 総合エラー率: < 1%
- HTTPステータス別: 2xx > 99%, 409/422/5xx < 1%

**分析ポイント**:
- バッチ処理のオーバーヘッド
- ベースラインとしての参照値

---

### Medium (100エントリ)

**目的**: 中規模バッチの性能特性を測定。実用的なバッチサイズの評価。

**実行方法**:

```bash
# 注: tasks_bulk_100.lua は今後実装予定
wrk2 -t2 -c10 -d30s -R100 -s scripts/tasks_bulk.lua http://localhost:3002
```

**期待される結果**:
- RPS: 60-100 req/s（-R100 の固定レート以下）
- P50 レイテンシ: < 200ms
- P95 レイテンシ: < 500ms
- P99 レイテンシ: < 1秒
- 総合エラー率: < 1%
- HTTPステータス別: 2xx > 99%, 409/422/5xx < 1%

**分析ポイント**:
- Small との比較でスケーリング特性を把握
- 10倍のエントリ数で RPS がどの程度低下するか

---

### Large (1,000エントリ)

**目的**: 大規模バッチの性能特性を測定。ホットスポット分析の主要対象。

**実行方法**:

```bash
# 注: tasks_bulk_1000.lua は今後実装予定
wrk2 -t2 -c10 -d30s -R50 -s scripts/tasks_bulk.lua http://localhost:3002
```

**期待される結果**:
- RPS: 30-50 req/s（-R50 の固定レート以下）
- P50 レイテンシ: < 2秒
- P95 レイテンシ: < 5秒
- P99 レイテンシ: < 10秒
- 総合エラー率: < 5%
- HTTPステータス別: 2xx > 95%, 409/422/5xx < 5%

**分析ポイント**:
- メモリアロケーション、コピー処理のボトルネック
- 永続データ構造の更新コスト
- プロファイリングによるホットスポット特定

**プロファイリング推奨**:

```bash
# perf によるプロファイリング（注: スクリプトは今後実装予定）
# PROFILE_MODE=true ./run_benchmark.sh --scenario scenarios/tasks_bulk_1000.yaml
```

---

### XLarge (10,000エントリ) - 条件付き

**目的**: 超大規模バッチの性能特性とメモリ使用量の測定。

**実行条件**:
1. Large (1,000エントリ) の測定が正常完了
2. エラー率 < 10%
3. P99 レイテンシ < 60秒

**実行方法**:

```bash
# 注: 自動打ち切りスクリプトは今後実装予定
# ./scripts/run_tasks_bulk_scale.sh --max-scale xlarge

# 手動実行
# 注: tasks_bulk_10000.lua は今後実装予定
wrk2 -t2 -c10 -d30s -R10 -s scripts/tasks_bulk.lua http://localhost:3002
```

**期待される結果**:
- RPS: 5-10 req/s（-R10 の固定レート以下）
- P50 レイテンシ: < 5秒
- P95 レイテンシ: < 15秒
- P99 レイテンシ: 10-30秒
- 総合エラー率: < 10%
- HTTPステータス別: 2xx > 90%, 409 < 8%, 422/5xx < 2%

**警告条件**:
- P99 > 60秒: 次のスケール（XXLarge）をスキップ
- エラー率 > 10%: 次のスケールをスキップ

**分析ポイント**:
- メモリ使用量の急増
- GC 頻度とレイテンシへの影響
- タイムアウトのリスク

---

### XXLarge (100,000エントリ) - 条件付き

**目的**: 上限テスト。タイムアウト・OOM のリスク評価。

**実行条件** (全て満たす必要あり):

1. **前提条件**: Large (1,000) と XLarge (10,000) の測定が正常完了
   - エラー率 < 10%
   - タイムアウトなし

2. **推定完了時間**: < 60秒
   - 計算方法: `30秒（測定時間） × 安全係数 2 = 60秒`
   - XLarge の P99 レイテンシから推定: P99 が 30秒を超える場合、XXLarge は60秒を超える可能性が高い

3. **メモリ使用量**: < 利用可能メモリの 80%
   - `free -m` または `vm_stat` で確認
   - 推定メモリ使用量: 100,000エントリ × 平均500バイト ≈ 50MB（JSON ボディ）

4. **リクエストボディサイズ制限**:
   - benches/api の Axum 設定 `body_limit` >= 64MB を確認

**事前チェック**:

```bash
# 注: チェックスクリプトは今後実装予定
# ./scripts/run_tasks_bulk_scale.sh --max-scale xxlarge --check-only
# ./scripts/check_bulk_prerequisites.sh --scale 100000

# 手動でメモリとbody_limitを確認してから実行
```

**実行方法**:

```bash
# 注: 自動打ち切りスクリプトは今後実装予定
# ./scripts/run_tasks_bulk_scale.sh --max-scale xxlarge

# 手動実行（非推奨、リスクあり）
# 注: tasks_bulk_100000.lua は今後実装予定
wrk2 -t2 -c10 -d30s -R1 -s scripts/tasks_bulk.lua http://localhost:3002
```

**期待される結果**:
- RPS: 0.5-1 req/s（-R1 の固定レート以下、1リクエスト 1-2秒）
- P50 レイテンシ: < 10秒
- P95 レイテンシ: < 30秒
- P99 レイテンシ: < 60秒
- 総合エラー率: < 10%
- HTTPステータス別: 2xx > 90%, 409 < 8%, 422/5xx < 2%

**自動打ち切り基準**:
- **P99 > 60秒**: 警告を出して停止
- **エラー率 > 10%**: 警告を出して停止
- **OOM 検出**: 即時停止

**実行しない場合の記録**:

測定が実行されない場合、`skip_reason` を記録します。

```json
{
  "scale": 100000,
  "executed": false,
  "skip_reason": "Estimated P99 latency exceeds limit (60s)",
  "prerequisites_met": {
    "large_success": true,
    "xlarge_success": true,
    "estimated_time_ok": false,
    "memory_ok": true,
    "body_limit_ok": true
  }
}
```

**分析ポイント**:
- システムの上限を把握
- OOM のリスク評価
- 実用的なバッチサイズの上限決定

---

## 実行方法

### 自動スケールアップスクリプト（推奨）

全スケールを自動的に実行し、条件付きスケールは自動判定します。

```bash
cd benches/api/benchmarks

# 注: 自動スケールアップスクリプトは今後実装予定
# 現時点では手動で各スケールを実行する必要があります

# Small から XLarge まで実行（将来の実装）
# ./scripts/run_tasks_bulk_scale.sh --max-scale xlarge

# Small から XXLarge まで実行（条件付き、将来の実装）
# ./scripts/run_tasks_bulk_scale.sh --max-scale xxlarge

# 事前チェックのみ実行（将来の実装）
# ./scripts/run_tasks_bulk_scale.sh --max-scale xxlarge --check-only
```

**オプション**:
- `--max-scale <scale>`: 最大スケールを指定（small, medium, large, xlarge, xxlarge）
- `--check-only`: 事前チェックのみ実行し、実測定はスキップ
- `--seed <number>`: 乱数シードを指定（デフォルト: 42）
- `--output <dir>`: 結果の出力先ディレクトリ

### 手動実行

各スケール個別に手動実行する場合。

```bash
# 注1: SEED は test_ids.lua 専用で、ペイロード生成には影響しません
# 注2: 各スケール専用のLuaスクリプトは今後実装予定
# 現時点では tasks_bulk.lua のバッチサイズを変更して使用

# Small（将来: tasks_bulk_10.lua）
wrk2 -t2 -c10 -d30s -R100 -s scripts/tasks_bulk.lua http://localhost:3002

# Medium（将来: tasks_bulk_100.lua）
wrk2 -t2 -c10 -d30s -R100 -s scripts/tasks_bulk.lua http://localhost:3002

# Large（将来: tasks_bulk_1000.lua）
wrk2 -t2 -c10 -d30s -R50 -s scripts/tasks_bulk.lua http://localhost:3002

# XLarge（条件付き、将来: tasks_bulk_10000.lua）
wrk2 -t2 -c10 -d30s -R10 -s scripts/tasks_bulk.lua http://localhost:3002

# XXLarge（条件付き、事前チェック必須、将来: tasks_bulk_100000.lua）
wrk2 -t2 -c10 -d30s -R1 -s scripts/tasks_bulk.lua http://localhost:3002
```

---

## 自動打ち切り基準

安全性を確保するため、以下の基準で自動的に測定を打ち切ります。

### 警告レベル（次のスケールをスキップ）

1. **P99 > 60秒**
   - タイムアウトのリスクが高い
   - 次のスケールはスキップ

2. **エラー率 > 10%**
   - システムが限界に達している
   - 次のスケールはスキップ

### エラーレベル（即時停止）

1. **OOM 検出**
   - API サーバーのログに OOM が記録された
   - 即座に測定を停止し、結果を記録

2. **連続タイムアウト**
   - 3回連続でタイムアウトが発生
   - 測定を停止し、警告を出力

---

## 事前チェック項目

XXLarge (100,000エントリ) の実行前に以下をチェックします。

### 1. リクエストボディサイズ制限

**確認先**: `benches/api/src/main.rs` または Axum 設定

```rust
// body_limit の確認
.layer(DefaultBodyLimit::max(64 * 1024 * 1024)) // 64MB
```

**推定サイズ**: 100,000エントリ × 平均500バイト ≈ 50MB

**対策**: 不足する場合は `body_limit` を 128MB に増やす。

### 2. タイムアウト設定

**wrk2 のデフォルトタイムアウト**: 2秒（`--timeout`オプションで変更可能）

**API サーバーのタイムアウト**: 確認が必要（設定ファイルやコードを参照）

**推奨**:
- wrk2: タイムアウトを延長（`--timeout 120s` など）
- API: リクエストタイムアウトを 120秒以上に設定

### 3. メモリ使用量

**確認コマンド**:

```bash
# Linux
free -m

# macOS
vm_stat | grep "Pages free"
```

**判定基準**:
- メモリ使用量 < 利用可能メモリの 80%
- 推定使用量（50MB ボディ + 処理用メモリ）を考慮

### 4. 前提スケールの成功

**確認項目**:
- Large (1,000) のエラー率 < 10%
- XLarge (10,000) のエラー率 < 10%
- XLarge の P99 < 60秒

---

## 結果の解釈方法

### スケーリング特性の分析

各スケール間の RPS とレイテンシの変化から、スケーリング特性を評価します。

**理想的なスケーリング**:
- エントリ数が10倍になっても RPS は 1/10 未満の低下
- P99 レイテンシは線形に増加

**非線形な退行**:
- エントリ数が10倍で RPS が 1/20 以下に低下 → ボトルネック
- P99 レイテンシが指数的に増加 → メモリやGCの問題

**例**:

| スケール | エントリ数 | RPS | P99 (秒) | スケーリング比 |
|---------|-----------|-----|----------|--------------|
| Small | 10 | 100 | 0.1 | - |
| Medium | 100 | 80 | 0.15 | 0.8x (良好) |
| Large | 1,000 | 50 | 2.0 | 0.625x (良好) |
| XLarge | 10,000 | 10 | 20.0 | 0.2x (退行) |

**解釈**:
- Small → Large: 良好なスケーリング
- Large → XLarge: RPS が 1/5 に低下（非線形な退行）
- ボトルネック: 10,000エントリ付近にボトルネックが存在

### エラーの分析

エラー率とHTTPステータスコードから、問題の種類を特定します。

**HTTPステータス別の意味**:

| ステータス | 意味 | 対策 |
|-----------|------|------|
| 409 Conflict | 楽観的ロック競合 | 仕様内動作、リトライで解決 |
| 422 Validation Error | バリデーションエラー | 入力データの確認 |
| 500 Server Error | サーバー内部エラー | バグの可能性、ログ確認 |
| 503 Service Unavailable | サーバー過負荷 | レート制御、スケールダウン |

**エラー率の判断基準**:
- < 1%: 正常
- 1-5%: 注意（原因確認）
- 5-10%: 警告（次のスケールは慎重に）
- > 10%: エラー（次のスケールをスキップ）

### メモリ使用量の分析

各スケールでのメモリ使用量から、メモリリークやGCの問題を検出します。

**確認方法**:

```bash
# API サーバーのメモリ使用量を監視
docker stats api-server --no-stream

# または
ps aux | grep api-server
```

**分析ポイント**:
- メモリ使用量が線形に増加: 正常
- メモリ使用量が指数的に増加: メモリリークの可能性
- メモリ使用量が急増後、急減: GC が頻発

---

## トラブルシューティング

### P99 レイテンシが異常に高い

**原因**:
- GC の頻発
- メモリスワップの発生
- データ構造のコピーコスト

**対策**:
1. プロファイリングでホットスポットを特定
2. メモリ使用量を確認
3. GC ログを確認

### エラー率が高い（> 10%）

**原因**:
- システム限界を超えている
- タイムアウト
- OOM

**対策**:
1. HTTPステータス別にエラーを分類
2. API サーバーのログを確認
3. スケールを下げて再測定

### OOM が発生

**原因**:
- バッチサイズが大きすぎる
- メモリリーク

**対策**:
1. バッチサイズを下げる（例: 100,000 → 10,000）
2. メモリ使用量を監視しながら再測定
3. メモリリークの調査（Valgrind, heaptrack など）

### 測定が収束しない（CI幅 > 10%）

**原因**:
- 環境ノイズ（バックグラウンドプロセス、CPU周波数変動）
- キャッシュヒット率の変動

**対策**:
1. [環境ノイズ対策ガイド](./environment_setup.md) を参照
2. 測定時間を延長（`-d 60s` など）
3. ウォームアップ時間を延長

---

## CI/CD での使用

### GitHub Actions での例

```yaml
# 注: 以下のスクリプトは今後実装予定
# 現時点ではプレースホルダーとして記載

- name: Run tasks_bulk scale benchmark
  run: |
    cd benches/api/benchmarks
    # 将来の実装: ./scripts/run_tasks_bulk_scale.sh \
    #   --max-scale xlarge \
    #   --output results/${{ github.sha }}/tasks_bulk

- name: Analyze results
  run: |
    cd benches/api/benchmarks
    # 将来の実装: python3 scripts/analyze_scale_results.py \
    #   results/${{ github.sha }}/tasks_bulk

- name: Upload results
  uses: actions/upload-artifact@v3
  with:
    name: tasks-bulk-scale-results
    path: benches/api/benchmarks/results/${{ github.sha }}/tasks_bulk
```

---

## 参考資料

- [ベンチマーク網羅性改善 要件定義](../../../docs/internal/requirements/20260201_1300_benchmark_coverage_improvement.yaml)
- [統計結果フォーマット定義](./stats_format.md)
- [環境ノイズ対策ガイド](./environment_setup.md)
- [tasks_bulk ボトルネック改善](../../../docs/internal/requirements/20260201_1120_tasks_bulk_bottleneck_remediation.yaml)
