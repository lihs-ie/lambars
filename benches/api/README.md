# Task Management Benchmark API

lambarsライブラリの機能を使用するタスク管理API。
HTTP APIとして実装し、負荷テストで性能を測定することを目的としています。

## 概要

このAPIは、lambarsの以下の機能をHTTPエンドポイントとして公開し、
実際のアプリケーションに近い形でパフォーマンスを測定することを目的としています。

**現在の実装状況**: 基盤のみ実装済み。エンドポイントは順次追加予定。

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│                    HTTP Clients                         │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                      API Layer                          │
│             (Axum Handlers + lambars)                   │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                   Domain Layer                          │
│          (Task, Project, History models)                │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                Infrastructure Layer                     │
│   ┌────────────┐ ┌────────────┐ ┌──────────────────┐    │
│   │  InMemory  │ │  Postgres  │ │      Redis       │    │
│   │ Repository │ │ Repository │ │   (Cache)        │    │
│   └────────────┘ └────────────┘ └──────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## クイックスタート

### Docker環境での起動

```bash
cd benches/api/docker

# 全サービスを起動（PostgreSQL + Redis + API）
docker compose up -d

# APIのヘルスチェック（Docker環境はポート3002）
curl http://localhost:3002/health

# タスク作成
curl -X POST http://localhost:3002/tasks \
  -H "Content-Type: application/json" \
  -d '{"title": "Sample Task", "description": "A test task"}'

# ログを確認
docker compose logs -f api

# 停止
docker compose down

# データも含めてクリーンアップ
docker compose down -v
```

### 開発モードでの起動

```bash
cd benches/api/docker

# 開発プロファイルで起動（ホットリロード対応、ポート3001）
docker compose --profile dev up -d api-dev

curl http://localhost:3001/health
```

### ローカル環境での起動（InMemoryモード）

PostgreSQL/Redisなしで実行できます:

```bash
cd benches/api

# InMemoryモードで起動（デフォルト、ポート3000）
cargo run

curl http://localhost:3000/health
```

### ローカル環境での起動（DB接続あり）

```bash
cd benches/api

# Docker ComposeでDB/Redisのみ起動
cd docker && docker compose up -d postgres redis && cd ..

# 環境変数を設定
export DATABASE_URL=postgres://benchmark:benchmark@localhost:5433/benchmark
export REDIS_URL=redis://localhost:6380
export STORAGE_MODE=postgres
export CACHE_MODE=redis

cargo run
```

## 環境変数

### 基本設定

| 変数 | デフォルト | 説明 |
|------|-----------|------|
| `HOST` | `0.0.0.0` | サーバーのバインドアドレス |
| `PORT` | `3000` | サーバーのポート |
| `STORAGE_MODE` | `in_memory` | ストレージ: `in_memory`, `postgres` |
| `CACHE_MODE` | `in_memory` | キャッシュ: `in_memory`, `redis` |
| `DATABASE_URL` | - | PostgreSQL接続URL（`STORAGE_MODE=postgres`時に必要） |
| `REDIS_URL` | - | Redis接続URL（`CACHE_MODE=redis`時に必要） |
| `RUST_LOG` | `info` | ログレベル |

### キャッシュ設定（`CACHE_MODE=redis`時）

| 変数 | デフォルト | 説明 |
|------|-----------|------|
| `CACHE_ENABLED` | `true` | キャッシュの有効/無効 |
| `CACHE_STRATEGY` | `read-through` | キャッシュ戦略: `read-through`, `write-through` |
| `CACHE_TTL_SECS` | `60` | キャッシュTTL（秒） |

**キャッシュ戦略の説明**:
- `read-through`: 読み取り時にキャッシュミスの場合、主ストレージから取得してキャッシュに書き込み
- `write-through`: read-through + 書き込み時にキャッシュを同期更新

**CACHE_ENABLED=false の場合**:
- 読み取り: Redis をバイパスし、主ストレージを直接参照
- 書き込み: 主ストレージ更新後、Redis キャッシュを無効化（再有効化時の整合性保証）

## ポート一覧

| 起動方法 | APIポート | 説明 |
|---------|----------|------|
| ローカル | 3000 | `cargo run` |
| Docker (dev) | 3001 | `docker compose --profile dev up` |
| Docker (prod) | 3002 | `docker compose up` |

## キャッシュヘッダー

`CACHE_MODE=redis` の場合、キャッシュ対象エンドポイントのレスポンスには以下のヘッダーが付与されます:

| ヘッダー | 値 | 説明 |
|---------|-----|------|
| `X-Cache` | `HIT` / `MISS` | キャッシュヒット/ミス |
| `X-Cache-Status` | `hit` / `miss` / `bypass` / `error` | 詳細ステータス |
| `X-Cache-Source` | `redis` / `memory` / `none` | キャッシュソース |

**X-Cache-Status の値**:
- `hit`: キャッシュから取得
- `miss`: キャッシュになく主ストレージから取得
- `bypass`: `CACHE_ENABLED=false` でバイパス
- `error`: Redis 障害でフォールバック

**キャッシュ対象エンドポイント**:
- `GET /tasks/{id}` - 単一タスク取得（Redis）
- `GET /projects/{id}` - 単一プロジェクト取得（Redis）
- `GET /tasks/search` - タスク検索（in-memory SearchCache）

**キャッシュ非対象エンドポイント**（X-Cache ヘッダーなし）:
- 一覧系: `GET /tasks`, `GET /projects`
- 集約系: `/dashboard`, `/projects/leaderboard`
- 書き込み系: `POST`, `PUT`, `DELETE`

## APIエンドポイント

### 実装済み

| メソッド | パス | 説明 |
|----------|------|------|
| `GET` | `/health` | ヘルスチェック |
| `POST` | `/tasks` | タスク作成（Functor, Monad, Either使用） |

### 計画中

以下のエンドポイントは順次実装予定です:

| メソッド | パス | lambars機能 | 説明 |
|----------|------|------------|------|
| `POST` | `/tasks-eff` | ExceptT, AsyncIO, eff_async! | タスク作成（Effect版） |
| `GET` | `/tasks` | PersistentVector, Traversable | タスク一覧 |
| `GET` | `/tasks/{id}` | Optional, Either | タスク取得 |
| `PUT` | `/tasks/{id}` | Lens, Optional, Bifunctor | タスク更新 |
| `PATCH` | `/tasks/{id}/status` | Prism, Either | ステータス更新 |
| `GET` | `/tasks/search` | PersistentTreeMap, Alternative | 検索 |
| `POST` | `/tasks/bulk` | Alternative, Bifunctor, for_! | 一括作成 |
| `POST` | `/projects` | Applicative, Validated | プロジェクト作成 |
| `GET` | `/projects/{id}/progress` | Foldable, Trampoline, Monoid | 進捗計算 |

詳細は `docs/internal/requirements/20260118_1520_task_management_benchmark_api.yaml` を参照してください。

## 負荷テスト

### curlによる基本テスト

```bash
# Docker環境（ポート3002）での例

# ヘルスチェック
curl http://localhost:3002/health

# タスク作成
curl -X POST http://localhost:3002/tasks \
  -H "Content-Type: application/json" \
  -d '{"title": "Benchmark Task", "description": "Test task for benchmarking"}'
```

### wrk/wrk2による負荷テスト

wrk2を使用した負荷テストが利用可能です。

```bash
cd benches/api/benchmarks

# 基本的な負荷テスト（シナリオYAMLを使用）
./run_benchmark.sh scenarios/read_heavy_warm.yaml

# 負荷プロファイル付きテスト（wrk2推奨）
wrk2 -t4 -c30 -d60s -R500 -s scripts/load_shape_demo.lua \
    --latency http://localhost:3002 \
    -- --profile=ramp_up_down --payload=standard --target-rps=500

# シナリオマトリクスによるテスト
./run_benchmark.sh scenarios/mixed_workload_burst.yaml
./run_benchmark.sh scenarios/large_scale_read.yaml
```

シナリオYAMLで設定可能な項目:
- `storage_mode`: `in_memory`, `postgres`
- `cache_mode`: `in_memory`, `redis`
- `load_pattern`: `read_heavy`, `write_heavy`, `mixed`
- `rps_profile`: `constant`, `ramp_up_down`, `burst`, `step_up`
- `payload_variant`: `minimal`, `standard`, `complex`, `heavy`
- `data_scale`: `small`, `medium`, `large`

詳細は `benchmarks/scenarios/` 内のYAMLファイルを参照してください。

## ディレクトリ構成

```
benches/api/
├── Cargo.toml
├── README.md                    # このファイル
├── src/
│   ├── main.rs                  # エントリーポイント
│   ├── lib.rs                   # ライブラリルート
│   ├── api/                     # API層
│   │   ├── mod.rs
│   │   ├── handlers.rs          # HTTPハンドラ
│   │   ├── dto.rs               # リクエスト/レスポンス型
│   │   ├── error.rs             # エラー型
│   │   └── cache_header.rs      # X-Cacheヘッダーミドルウェア
│   ├── domain/                  # ドメイン層
│   │   ├── mod.rs
│   │   ├── task.rs              # タスクモデル
│   │   ├── project.rs           # プロジェクトモデル
│   │   └── history.rs           # 履歴モデル
│   └── infrastructure/          # インフラ層
│       ├── mod.rs
│       ├── repository.rs        # リポジトリトレイト
│       ├── factory.rs           # リポジトリファクトリ
│       ├── in_memory.rs         # InMemory実装
│       ├── postgres.rs          # PostgreSQL実装
│       ├── redis.rs             # Redis実装
│       ├── cache.rs             # CacheRepository（read-through/write-through）
│       └── scenario.rs          # ベンチマークシナリオ設定
├── benchmarks/                  # 負荷テスト
│   ├── run_benchmark.sh         # ベンチマーク実行スクリプト
│   ├── scenarios/               # シナリオYAML
│   │   ├── matrix.yaml          # シナリオマトリクス定義
│   │   ├── read_heavy_warm.yaml # 読み取り負荷テスト
│   │   └── ...
│   ├── scripts/                 # Luaスクリプト
│   │   ├── common.lua           # 共通ユーティリティ
│   │   ├── load_profile.lua     # 負荷プロファイル
│   │   ├── cache_metrics.lua    # キャッシュ計測
│   │   └── ...
│   └── templates/               # シナリオテンプレート
└── docker/
    ├── compose.yaml             # Docker Compose設定
    ├── Dockerfile               # APIコンテナ
    └── postgres/
        └── init.sql             # DB初期化スクリプト
```

## CI/CD

このAPIベンチマークはCIでは実行されません（DB/Redis依存でノイズが大きいため）。
手動実行または夜間スケジュールジョブでの実行を推奨します。

高精度な回帰検出には、`benches/iai/` のIai-Callgrindベンチマークを使用してください。

## 参考資料

- [Axum - Web framework](https://docs.rs/axum)
- [SQLx - Async SQL toolkit](https://docs.rs/sqlx)
- [lambars Documentation](../../README.md)
- [実装計画](../../docs/internal/plans/20260118_1500_continuous_benchmark_roadmap.yaml)
