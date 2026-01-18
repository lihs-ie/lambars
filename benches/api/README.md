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

| 変数 | デフォルト | 説明 |
|------|-----------|------|
| `HOST` | `0.0.0.0` | サーバーのバインドアドレス |
| `PORT` | `3000` | サーバーのポート |
| `STORAGE_MODE` | `in_memory` | ストレージ: `in_memory`, `postgres` |
| `CACHE_MODE` | `in_memory` | キャッシュ: `in_memory`, `redis` |
| `DATABASE_URL` | - | PostgreSQL接続URL（`STORAGE_MODE=postgres`時に必要） |
| `REDIS_URL` | - | Redis接続URL（`CACHE_MODE=redis`時に必要） |
| `RUST_LOG` | `info` | ログレベル |

## ポート一覧

| 起動方法 | APIポート | 説明 |
|---------|----------|------|
| ローカル | 3000 | `cargo run` |
| Docker (dev) | 3001 | `docker compose --profile dev up` |
| Docker (prod) | 3002 | `docker compose up` |

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

### wrkによる負荷テスト（将来対応）

エンドポイントの実装が完了次第、wrk + Luaスクリプトによる負荷テストを追加予定です。

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
│   │   └── error.rs             # エラー型
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
│       └── redis.rs             # Redis実装
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
