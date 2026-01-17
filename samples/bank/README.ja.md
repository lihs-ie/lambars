# Bank サンプルアプリケーション

[English](README.md)

[lambars](../../docs/external/readme/README.ja.md) を使用した関数型プログラミングパターンを示す、包括的な Event Sourcing / CQRS サンプルアプリケーションです。

## 概要

このサンプルは、Rust で関数型プログラミングの原則を使用して本番品質のアプリケーションを構築する方法を示すバンキング API を実装しています。以下を実演します：

- **Event Sourcing**: すべての状態変更を不変のイベントとして記録
- **CQRS**: 最適なパフォーマンスのための読み取りと書き込みモデルの分離
- **関数型ドメインモデリング**: 副作用のない純粋なビジネスロジック
- **包括的なエラーハンドリング**: `Either` と `Validated` 型の使用

## アーキテクチャ

アプリケーションはオニオンアーキテクチャに従います：

```
┌─────────────────────────────────────────────────────────────┐
│                       API レイヤー                           │
│  (HTTP ハンドラー、DTO、ミドルウェア、ルーティング)             │
├─────────────────────────────────────────────────────────────┤
│                  アプリケーションレイヤー                      │
│  (ワークフロー、バリデーション、クエリ、サービス)                │
├─────────────────────────────────────────────────────────────┤
│                     ドメインレイヤー                          │
│  (集約、イベント、コマンド、値オブジェクト)                     │
├─────────────────────────────────────────────────────────────┤
│                 インフラストラクチャレイヤー                   │
│  (Event Store、Read Model、メッセージング、設定)              │
└─────────────────────────────────────────────────────────────┘
```

### ディレクトリ構造

```
src/
├── api/                    # HTTP API レイヤー
│   ├── dto/                # リクエスト/レスポンス DTO
│   ├── handlers/           # Axum ルートハンドラー
│   │   ├── account.rs      # アカウント操作
│   │   ├── transaction.rs  # トランザクション操作
│   │   ├── pipeline.rs     # パイプラインユーティリティ
│   │   └── workflow_eff.rs # eff_async! パターンユーティリティ
│   ├── middleware/         # エラーハンドリングミドルウェア
│   └── routes.rs           # ルート設定
├── application/            # アプリケーションレイヤー
│   ├── validation/         # 入力バリデーション
│   ├── workflows/          # ビジネスワークフロー
│   ├── queries/            # CQRS クエリ
│   └── services/           # アプリケーションサービス
├── domain/                 # ドメインレイヤー
│   ├── account/            # Account 集約
│   │   ├── aggregate.rs    # Account エンティティ
│   │   ├── commands.rs     # コマンド定義
│   │   ├── events.rs       # イベント定義
│   │   └── errors.rs       # ドメインエラー
│   ├── value_objects/      # 値オブジェクト
│   ├── audit/              # 監査ログ
│   └── validation/         # Validated 型
└── infrastructure/         # インフラストラクチャレイヤー
    ├── event_store.rs      # イベント永続化
    ├── read_model.rs       # Read Model キャッシュ
    ├── messaging.rs        # イベント発行
    └── config.rs           # 設定
```

## 使用している lambars の機能

このサンプルは、さまざまな [lambars](../../docs/external/readme/README.ja.md) 機能の実践的な使用を示しています：

### Phase 1: コア関数型パターン

| 機能 | 用途 | 例 |
|------|------|-----|
| `Either<L, R>` | ドメインロジックでのエラーハンドリング | `Either<DomainError, Account>` |
| `Semigroup` | Money 値の合成 | `money1.combine(money2)` |
| `Monoid` | Money 操作の単位元 | `Money::empty()` |
| [`Trampoline`](../../docs/external/readme/README.ja.md#trampolineスタック安全な再帰) | スタック安全なイベントリプレイ | `replay_events(events)` |
| [`PersistentList`](../../docs/external/readme/README.ja.md#persistentlist) | 不変イベントシーケンス | イベントストレージ |

### Phase 2: パイプラインユーティリティ

```rust
use bank::api::handlers::pipeline::*;

// 非同期パイプライン合成
let result = async_pipe!(
    validate_input(request),
    build_command,
    execute_workflow
)?;

// 並列バリデーション
let validated = parallel_validate!(
    validate_name(name),
    validate_amount(amount),
    validate_currency(currency)
)?;
```

### Phase 3: ExceptT を使った eff_async! マクロ

このサンプルでは、非同期ハンドラーを記述するための2つのスタイルを提供しています：

#### 従来のスタイル（? 演算子）

```rust
pub async fn deposit_handler(
    State(deps): State<AppDependencies>,
    Path(id): Path<String>,
    Json(request): Json<DepositRequest>,
) -> Result<Json<Response>, ApiError> {
    let events = deps.event_store()
        .load_events(&id)
        .run_async()
        .await
        .map_err(|e| event_store_error(&e))?;

    let account = Account::from_events(&events)
        .ok_or_else(|| not_found_error())?;

    let event = deposit(&command, &account, timestamp)
        .map_err(|e| domain_error(&e))?;

    deps.event_store()
        .append_events(&id, vec![event.clone()])
        .run_async()
        .await
        .map_err(|e| event_store_error(&e))?;

    Ok(Json(response))
}
```

#### eff_async! スタイル（do 記法）

```rust
use lambars::eff_async;
use bank::api::handlers::workflow_eff::*;

async fn execute_workflow(
    command: &DepositCommand,
    account: &Account,
    event_store: &EventStore,
    timestamp: Timestamp,
) -> Result<MoneyDeposited, ApiError> {
    let workflow: WorkflowResult<MoneyDeposited> = eff_async! {
        event <= from_result(deposit(command, account, timestamp).map_err(domain_error));
        _ <= lift_async_result(event_store.append_events(&id, vec![event.clone()]), event_store_error);
        _ <= lift_async_result(read_model.invalidate(&id).fmap(Ok::<_, ()>), |_| internal_error());
        pure_async(event)
    };

    workflow.run_async_io().run_async().await
}
```

### Phase 5: 並列バリデーションのための Validated

```rust
use bank::domain::validation::Validated;

// フェイルファストではなく、すべてのバリデーションエラーを蓄積
let result: Validated<Vec<ValidationError>, Account> = Validated::map3(
    validate_owner_name(name),
    validate_initial_balance(balance),
    validate_currency(currency),
    |name, balance, currency| Account::new(name, balance, currency)
);

match result {
    Validated::Valid(account) => Ok(account),
    Validated::Invalid(errors) => Err(ValidationErrors(errors)),
}
```

### Phase 6: 監査ログのための Writer モナド

```rust
use lambars::effect::Writer;
use bank::application::workflows::audited::*;

// 監査ログを蓄積するワークフロー
let audited_workflow: Writer<Vec<AuditEntry>, MoneyDeposited> =
    audited_deposit(&command, &account, timestamp);

let (event, audit_logs) = audited_workflow.run();

// 監査ログには以下が含まれます：
// - タイムスタンプ
// - 操作タイプ
// - アクター情報
// - 変更前/変更後の状態
```

## API エンドポイント

| メソッド | パス | 説明 |
|----------|------|------|
| `POST` | `/accounts` | 新しいアカウントを作成 |
| `GET` | `/accounts/:id` | アカウント情報を取得 |
| `GET` | `/accounts/:id/balance` | アカウント残高を取得 |
| `POST` | `/accounts/:id/deposit` | 入金（従来のスタイル） |
| `POST` | `/accounts/:id/deposit-eff` | 入金（eff_async!） |
| `POST` | `/accounts/:id/withdraw` | 出金（従来のスタイル） |
| `POST` | `/accounts/:id/withdraw-eff` | 出金（eff_async!） |
| `POST` | `/accounts/:id/transfer` | アカウント間の送金 |
| `GET` | `/accounts/:id/transactions` | 取引履歴を取得 |
| `GET` | `/health` | ヘルスチェック |

## アプリケーションの実行

### 前提条件

- Rust 1.92.0 以降
- Docker（インフラストラクチャサービス用）

### クイックスタート

```bash
# インフラストラクチャサービスを起動
cd docker
docker compose up -d

# アプリケーションを実行
cargo run

# API は http://localhost:3000 で利用可能になります
```

### テストの実行

```bash
# すべてのテストを実行
cargo test

# 統合テストのみ実行
cargo test --test integration_tests

# 出力付きで実行
cargo test -- --nocapture
```

## 使用例

### アカウントの作成

```bash
curl -X POST http://localhost:3000/accounts \
  -H "Content-Type: application/json" \
  -d '{
    "owner_name": "Alice",
    "initial_balance": 10000,
    "currency": "JPY"
  }'
```

### 入金

```bash
curl -X POST http://localhost:3000/accounts/{id}/deposit \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: unique-key-123" \
  -d '{
    "amount": 5000,
    "currency": "JPY"
  }'
```

### 送金

```bash
curl -X POST http://localhost:3000/accounts/{from_id}/transfer \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: unique-key-456" \
  -d '{
    "to_account_id": "{to_id}",
    "amount": 3000,
    "currency": "JPY"
  }'
```

## 設計上の決定

### なぜ Event Sourcing か？

- **監査証跡**: すべての変更の完全な履歴
- **時間的クエリ**: 任意の時点の状態をクエリ可能
- **イベント駆動アーキテクチャ**: 他のシステムとの統合が容易
- **デバッグ**: イベントをリプレイして問題を再現

### なぜ関数型プログラミングか？

- **テスト容易性**: 純粋関数はテストが容易
- **合成可能性**: 小さな関数が複雑なワークフローに合成される
- **不変性**: 共有された可変状態がなく、並行コードがより安全
- **参照透過性**: 関数を単独で推論可能

### なぜ lambars か？

- **型安全なエフェクト**: IO と AsyncIO モナドが副作用を追跡
- **強力な抽象化**: Functor、Applicative、Monad によるクリーンな合成
- **永続データ構造**: 効率的な不変コレクション
- **Rust ネイティブ**: Rust の所有権モデル向けに特別に設計

## 既知の制限事項

### PersistentList が Send を実装していない

`PersistentList` は内部で `Rc` を使用しており、`Send` を実装していません。非同期ハンドラーでは、後続の `.await` 呼び出しの前に `PersistentList` の値がドロップされていることを確認する必要があります：

```rust
// 正しい: await の前に PersistentList をドロップ
let account = {
    let events = event_store.load_events(&id).run_async().await?;
    Account::from_events(&events)?
};  // events はここでドロップ

// 後続の await は安全
event_store.append_events(&id, new_events).run_async().await?;
```

詳細は [Issue: PersistentList not Send](../../docs/internal/issues/20260117_2100_persistent_list_not_send.yaml) を参照してください。

## 関連ドキュメント

- [lambars README](../../docs/external/readme/README.ja.md) - メインライブラリドキュメント
- [Haskell 比較](../../docs/external/comparison/Haskell/README.ja.md) - Haskell から lambars へのマッピング
- [要件定義](docs/internal/requirements/) - 詳細な要件定義
- [実装計画](docs/internal/done/plans/) - 完了した実装計画

## ライセンス

このサンプルは lambars の一部であり、同じ条件でライセンスされています：

- Apache License, Version 2.0
- MIT License

お好みで選択してください。
