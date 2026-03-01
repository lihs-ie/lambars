# Bank Sample API ベンチマーク

[English](README.md)

Bank Sample APIのパフォーマンスを評価するためのベンチマークスクリプトです。

## 前提条件

- [wrk](https://github.com/wg/wrk) - HTTP ベンチマークツール
- Docker 環境が起動していること（`docker compose up -d`）

### wrk のインストール

```bash
# macOS
brew install wrk

# Linux (Ubuntu/Debian)
apt install wrk

# Linux (ソースから)
git clone https://github.com/wg/wrk.git && cd wrk && make
```

## クイックスタート

```bash
# Docker 環境を起動
cd ../docker
docker compose up -d

# ベンチマーク実行（デフォルト: 4スレッド, 100接続, 30秒）
cd ../benchmarks
./run_benchmark.sh

# カスタム設定で実行
./run_benchmark.sh -t 8 -c 200 -d 60
```

## ベンチマークオプション

| オプション | デフォルト | 説明 |
|-----------|-----------|------|
| `-t, --threads` | 4 | スレッド数 |
| `-c, --connections` | 100 | 接続数 |
| `-d, --duration` | 30 | 実行時間（秒） |

## ベンチマーク対象

| エンドポイント | 説明 |
|---------------|------|
| `GET /health` | ヘルスチェック（ベースライン） |
| `POST /accounts/{id}/deposit` | 入金（従来スタイル） |
| `POST /accounts/{id}/deposit-eff` | 入金（eff_async! スタイル） |
| `POST /accounts/{id}/withdraw` | 出金（従来スタイル） |
| `POST /accounts/{id}/withdraw-eff` | 出金（eff_async! スタイル） |
| `POST /accounts/{id}/transfer` | 送金 |

## サンプル結果

環境: Apple M3 Pro, Docker Desktop 4.37, 2 CPU コア / 1GB メモリ制限

### スループット比較

| エンドポイント | リクエスト/秒 | 平均レイテンシ | p99 レイテンシ |
|---------------|--------------|---------------|---------------|
| ヘルスチェック（ベースライン） | ~41,000 | 5.08ms | 83.14ms |
| 入金（従来） | ~5,000 | 20.11ms | 49.23ms |
| 入金（eff_async!） | ~1,500 | 87.28ms | 689.98ms |
| 出金（従来） | ~1,100 | 89.05ms | 175.51ms |
| 出金（eff_async!） | ~850 | 144.71ms | 997.59ms |
| 送金 | ~970 | 102.61ms | 188.92ms |

### リソース使用量

| メトリック | 値 |
|-----------|-----|
| 最大 CPU | 87% |
| 最大メモリ | 1GB 制限の ~1% |

### 従来スタイル vs eff_async! スタイル

`eff_async!` スタイルのエンドポイントは従来の `?` 演算子スタイルと比較して高いレイテンシを示しています：

- **入金**: 従来スタイルが約3.4倍高速
- **出金**: 従来スタイルが約1.3倍高速

このオーバーヘッドはモナドトランスフォーマースタック（`ExceptT<ApiError, AsyncIO<...>>`）と追加の抽象化レイヤーに起因します。パフォーマンスが重要なパスでは、`?` 演算子を使用した従来スタイルを推奨します。

## 結果の理解

### なぜトランザクションエンドポイントはヘルスチェックより遅いのか？

1. **データベース操作**: 各トランザクションはPostgreSQLイベントストア操作を伴う
2. **イベントソーシング**: イベントストリームの読み書き
3. **並行性**: 同一アカウントでのロック競合

### なぜ eff_async! エンドポイントは遅いのか？

1. **モナドトランスフォーマーのオーバーヘッド**: `ExceptT` ラッピングによる関数呼び出しオーバーヘッド
2. **ボクシング**: エフェクト合成にヒープアロケーションが必要
3. **抽象化コスト**: クリーンな関数型合成には実行時コストがある

## 出力ファイル

結果は `results/` ディレクトリに保存されます：

- `benchmark_YYYYMMDD_HHMMSS.txt` - 詳細なベンチマーク出力
- `resources_YYYYMMDD_HHMMSS.csv` - リソース使用量の時系列データ

## 手動テスト

個別のスクリプトを手動で実行できます：

```bash
# ヘルスチェック
wrk -t4 -c100 -d10s -s scripts/health.lua http://localhost:8081

# 入金（アカウントIDが必要）
wrk -t4 -c100 -d10s -s scripts/deposit.lua http://localhost:8081 -- <account_id>

# 入金（eff_async! エンドポイント）
wrk -t4 -c100 -d10s -s scripts/deposit.lua http://localhost:8081 -- <account_id> eff

# 送金（2つのアカウントIDが必要）
wrk -t4 -c100 -d10s -s scripts/transfer.lua http://localhost:8081 -- <from_id> <to_id>
```

## リソース監視

ベンチマーク実行中に別途リソースを監視できます：

```bash
# リアルタイム監視
docker stats bank-app

# カスタムフォーマット
docker stats bank-app --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}"
```
