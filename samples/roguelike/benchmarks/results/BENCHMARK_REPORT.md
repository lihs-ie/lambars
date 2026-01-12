# roguelike API パフォーマンス評価レポート

## 実行環境

| 項目 | 値 |
|------|-----|
| 日時 | 2026-01-12 |
| Rust バージョン | 1.92.0 |
| OS | macOS Darwin 23.4.0 |
| Docker 構成 | roguelike-app, MySQL 8.0, Redis 7-alpine |

---

## エグゼクティブサマリー

### 全体評価: 良好

roguelike APIは全体的に良好なパフォーマンスを示しています。主要なエンドポイントはすべてSLO目標を大幅に下回るレイテンシで動作しています。

### 主要な結果

| 評価項目 | 結果 |
|---------|------|
| 全エンドポイントSLO達成 | **達成** |
| エラー率 | **0.00%** |
| 平均レスポンス時間 | **2.37ms** |
| p95レスポンス時間 | **8.99ms** |

### 推奨改善点

1. **FloorResponse のシリアライゼーション最適化** - 50x50マップで約17µsかかるため、大きなマップでは検討が必要
2. **キャッシュ戦略の活用** - ゲームセッションのキャッシュヒット率を監視・最適化

---

## k6 HTTPベンチマーク結果

### スモークテスト (1 VU, 30秒)

```
Scenario: SMOKE
Duration: 30 seconds
Virtual Users: 1
Total Requests: 280
Requests/sec: 9.08
Error Rate: 0.00%
Memory: 3.86-4.62 MiB
```

### 負荷テスト (10-20 VU, 8分)

```
Scenario: LOAD
Duration: 8 minutes
Virtual Users: 10-20 (ramping)
Total Iterations: ~2,500
Error Rate: 0.00%
Memory: 3.86-5.93 MiB
```

### ストレステスト (50-100 VU, 14分)

```
Scenario: STRESS
Duration: 14 minutes
Virtual Users: 50-100 (ramping)
Total Iterations: ~17,000
Error Rate: 0.00%
Memory: 6.16-15.94 MiB
```

### エンドポイント別レイテンシ

| エンドポイント | 平均 | p95 | SLO目標 | 達成 |
|---------------|------|-----|---------|------|
| Health Check | 1.80ms | 2.40ms | 100ms | OK |
| Create Game | 13.05ms | 19.20ms | 500ms | OK |
| Get Game | 1.65ms | 3.00ms | 300ms | OK |
| Get Player | 0.45ms | 1.05ms | 200ms | OK |
| Get Floor | 0.65ms | 1.00ms | 400ms | OK |
| Execute Command | 1.79ms | 4.00ms | 400ms | OK |
| Get Events | 2.55ms | 5.10ms | 200ms | OK |
| Leaderboard | 2.85ms | 4.90ms | 400ms | OK |

### ゲームライフサイクル全体

| メトリクス | 値 |
|-----------|-----|
| 平均時間 | 534.80ms |
| p95 | 542.15ms |
| 含まれる操作 | ゲーム作成、状態取得、移動コマンド×5、Wait、イベント取得、終了 |

---

## Criterion マイクロベンチマーク結果

### DTOシリアライゼーション

| 対象 | 平均時間 |
|------|----------|
| GameSessionResponse | 約500ns |
| PlayerDetailResponse | 約1µs |
| HealthResponse | 約100ns |
| FloorResponse (10x10) | 約3.5µs |
| FloorResponse (25x25) | 約15µs |
| FloorResponse (50x50) | 約58µs |
| TurnResultResponse (10 events) | 約2µs |
| TurnResultResponse (50 events) | 約8µs |
| TurnResultResponse (100 events) | 約15µs |
| LeaderboardResponse (10 entries) | 約2.3µs |
| LeaderboardResponse (50 entries) | 約10.9µs |
| LeaderboardResponse (100 entries) | 約20.5µs |

### DTOデシリアライゼーション

| 対象 | 平均時間 |
|------|----------|
| GameSessionResponse | 494ns |
| HealthResponse | 137ns |
| FloorResponse (10x10) | 8.95µs |
| FloorResponse (25x25) | 44.62µs |
| FloorResponse (50x50) | 168.84µs |

---

## SLO達成状況

### サマリー

| SLO | 目標 | 実績 | 状態 |
|-----|------|------|------|
| http_req_duration (p95) | < 500ms | 8.99ms | OK |
| http_req_failed | < 1% | 0.00% | OK |
| roguelike_health_latency (p95) | < 100ms | 2.40ms | OK |
| roguelike_create_game_latency (p95) | < 500ms | 19.20ms | OK |
| roguelike_get_game_latency (p95) | < 300ms | 3.00ms | OK |
| roguelike_command_latency (p95) | < 400ms | 4.00ms | OK |

**全SLO達成率: 100%**

---

## ボトルネック分析

### 特定された潜在的ボトルネック

1. **FloorResponse のデシリアライゼーション**
   - 50x50マップで約170µsかかる
   - タイル配列の2次元構造が原因
   - 影響度: 低（クライアント側の処理時間に含まれる）

2. **ゲーム作成処理**
   - 平均13ms、p95で19ms
   - 初期状態生成とDB保存が主な要因
   - 影響度: 低（許容範囲内）

### ボトルネックではない箇所

- Health Check: 極めて高速（2.4ms以下）
- Get Game/Player: キャッシュ活用により高速（3ms以下）
- コマンド実行: 効率的な処理（4ms以下）

---

## 改善提案

### 高優先度

1. **大きなマップの最適化**
   - FloorResponseのストリーミング対応を検討
   - 可視領域のみを返すAPIの活用を推奨

### 中優先度

2. **キャッシュ戦略の監視**
   - Redis キャッシュヒット率のメトリクス追加
   - TTL設定の最適化

### 低優先度

3. **負荷テスト・ストレステストの定期実行**
   - CI/CDパイプラインへの統合
   - パフォーマンス回帰検出

---

## ベンチマーク実行コマンド

### k6 HTTPベンチマーク

```bash
# スモークテスト
k6 run -e SCENARIO=smoke benchmarks/k6/main.js

# 負荷テスト
k6 run -e SCENARIO=load benchmarks/k6/main.js

# ストレステスト
k6 run -e SCENARIO=stress benchmarks/k6/main.js
```

### Criterion マイクロベンチマーク

```bash
# シリアライゼーションベンチマーク
cargo bench --package roguelike-api --bench serialization_bench

# APIベンチマーク
cargo bench --package roguelike-api --bench api_bench
```

---

## メモリ使用量

### コンテナメモリ使用量

| コンテナ | アイドル時 | スモークテスト | 負荷テスト | ストレステスト | SLO | 状態 |
|---------|-----------|---------------|-----------|---------------|-----|------|
| roguelike-app | 3.86 MiB | 4.62 MiB | 5.93 MiB | 15.94 MiB | < 100MB | OK |
| roguelike-mysql | 555.9 MiB | - | - | - | - | - |
| roguelike-redis | 22.2 MiB | - | - | - | - | - |

### DTOメモリサイズ（スタック）

| DTO | サイズ |
|-----|--------|
| PositionResponse | 8 bytes |
| ResourceResponse | 8 bytes |
| TileResponse | 3 bytes |
| GameStatusResponse | 1 byte |
| PlayerResponse | 88 bytes |
| PlayerDetailResponse | 464 bytes |
| GameSessionResponse | 144 bytes |
| FloorResponse | 112 bytes |
| TurnResultResponse | 200 bytes |
| GameEventResponse | 80 bytes |
| LeaderboardResponse | 48 bytes |
| HealthResponse | 32 bytes |

### ヒープサイズ推定

| データ構造 | サイズ |
|-----------|--------|
| FloorResponse (10x10) | ~0.55 KB |
| FloorResponse (25x25) | ~2.44 KB |
| FloorResponse (50x50) | ~8.52 KB |
| FloorResponse (100x100) | ~31.66 KB |
| TurnResultResponse (10 events) | ~3.42 KB |
| TurnResultResponse (100 events) | ~32.42 KB |
| LeaderboardResponse (100 entries) | ~13.72 KB |

### メモリSLO達成状況

| SLO | 目標 | 実績 | 状態 |
|-----|------|------|------|
| コンテナアイドル時 | < 100MB | 3.86 MiB | OK |
| 負荷テスト時ピーク (10-20 VU) | < 500MB | 5.93 MiB | OK |
| ストレステスト時ピーク (50-100 VU) | < 500MB | 15.94 MiB | OK |
| メモリリーク | なし | なし | OK |

**評価**: roguelike-app のメモリ使用量は非常に効率的で、SLO を大幅に下回っています。100 VU のストレステストでも最大 15.94 MiB に抑えられており、高負荷時でもメモリ効率が良好です。テスト後もメモリ使用量が安定しており、メモリリークは検出されませんでした。

---

## メモリベンチマーク実行コマンド

```bash
# DTOメモリサイズ測定
cargo run --package roguelike-api --bin memory-bench

# メモリ監視付きk6テスト
./benchmarks/scripts/benchmark-with-memory.sh smoke

# コンテナメモリ状況確認
docker stats roguelike-app roguelike-mysql roguelike-redis --no-stream
```

---

## 次のステップ

- [x] 負荷テスト（10-20 VU）の実行
- [x] ストレステスト（50-100 VU）の実行
- [x] メモリリークテスト
- [ ] CI/CDへのベンチマーク統合
- [ ] パフォーマンス回帰検出の仕組み構築
- [ ] 本番環境に近い条件でのテスト

---

## Criterion ベンチマーク結果 (2026-01-12 更新)

### リクエストパース

| 対象 | 平均時間 |
|------|----------|
| CreateGameRequest (with seed) | 76.3 ns |
| CreateGameRequest (minimal) | 58.6 ns |
| EndGameRequest | 26.9 ns |
| ExecuteCommandRequest (move) | 136.8 ns |
| ExecuteCommandRequest (attack) | 159.6 ns |
| ExecuteCommandRequest (use_item) | 191.6 ns |
| ExecuteCommandRequest (wait) | 79.4 ns |
| ExecuteCommandRequest (equip) | 148.6 ns |
| ExecuteCommandRequest (unequip) | 133.8 ns |

### コマンドシリアライゼーション

| 対象 | 平均時間 |
|------|----------|
| Move command | 62.3 ns |
| Attack command | 69.9 ns |
| Wait command | 45.6 ns |
| Use item command | 82.1 ns |
| Pick up command | 47.8 ns |
| Drop command | 49.7 ns |
| Equip command | 49.4 ns |
| Unequip command | ~50 ns |

### クエリパラメータパース

| 対象 | 平均時間 |
|------|----------|
| GetEventsParams_from_pairs | 62.0 ns |
| GetLeaderboardParams_parse | 68.5 ns |

### UUIDパース

| 対象 | 平均時間 |
|------|----------|
| uuid_parse | 45.4 ns |
| uuid_new_v4 | 47.8 ns |
| uuid_to_string | 49.7 ns |

### DTOシリアライゼーション (更新)

| 対象 | 平均時間 |
|------|----------|
| GameSessionResponse | 457 ns |
| PlayerDetailResponse | 828 ns |
| HealthResponse | 102 ns |
| FloorResponse (10x10) | 6.16 µs |
| FloorResponse (25x25) | 32.4 µs |
| FloorResponse (50x50) | 155 µs |
| TurnResultResponse (10 events) | 3.14 µs |
| TurnResultResponse (50 events) | 12.9 µs |
| TurnResultResponse (100 events) | 28.6 µs |
| LeaderboardResponse (10 entries) | 2.24 µs |
| LeaderboardResponse (50 entries) | 10.2 µs |
| LeaderboardResponse (100 entries) | 20.0 µs |

### DTOデシリアライゼーション (更新)

| 対象 | 平均時間 |
|------|----------|
| GameSessionResponse | 529 ns |
| HealthResponse | 118 ns |
| FloorResponse (10x10) | 9.33 µs |
| FloorResponse (25x25) | 47.8 µs |
| FloorResponse (50x50) | 173 µs |

### パフォーマンス評価

| カテゴリ | 評価 |
|---------|------|
| リクエストパース | 優秀 (< 200ns) |
| コマンドシリアライゼーション | 優秀 (< 100ns) |
| UUIDパース | 優秀 (< 50ns) |
| DTO シリアライゼーション | 良好 |
| DTO デシリアライゼーション | 良好 |

**総合評価**: APIレイヤーのパフォーマンスは非常に良好です。リクエストパースとシリアライゼーションはマイクロ秒以下で完了し、全体的なAPIレイテンシへの影響は最小限です。

---

*Generated: 2026-01-12*
*Updated: 2026-01-12 - 負荷テスト・ストレステスト・メモリリークテスト追加*
*Updated: 2026-01-12 - Criterionマイクロベンチマーク結果追加*
