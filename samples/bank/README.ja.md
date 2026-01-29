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

## ユースケース別 lambars 機能の使用例

このサンプルでは、一般的なユースケースごとに整理した [lambars](../../docs/external/readme/README.ja.md) 機能の実践的な使用を示しています：

### ドメインロジックでのエラーハンドリング

特定のエラー型で失敗する可能性のある計算を表現するために [`Either<L, R>`](../../docs/external/readme/README.ja.md) を使用します。

```rust
use lambars::typeclass::Either;

// ドメイン関数は Result ではなく Either を返す
pub fn deposit(
    command: &DepositCommand,
    account: &Account,
    timestamp: Timestamp,
) -> Either<DomainError, MoneyDeposited> {
    if account.is_frozen() {
        Either::Left(DomainError::AccountFrozen)
    } else {
        Either::Right(MoneyDeposited::new(command.amount(), timestamp))
    }
}

// API 境界で Result に変換
let result = deposit(&command, &account, timestamp);
match result {
    Either::Right(event) => Ok(event),
    Either::Left(error) => Err(ApiError::from(error)),
}
```

### 金額の合成

型安全な値の合成のために [`Semigroup`](../../docs/external/readme/README.ja.md#semigroup-と-monoid) と [`Monoid`](../../docs/external/readme/README.ja.md#semigroup-と-monoid) を使用します。

```rust
use lambars::typeclass::{Semigroup, Monoid};

// Money は値を結合するために Semigroup を実装
let total = deposit1.amount().combine(deposit2.amount());

// Monoid は単位元を提供
let zero = Money::empty();  // 金額 0 の Money

// 複数の値を結合
let balance = transactions
    .iter()
    .fold(Money::empty(), |acc, tx| acc.combine(tx.amount()));
```

### スタック安全なイベントリプレイ

スタックオーバーフローなしで大量のイベントシーケンスを処理するために [`Trampoline`](../../docs/external/readme/README.ja.md#trampolineスタック安全な再帰) を使用します。

```rust
use lambars::control::Trampoline;

// 何千ものイベントを安全にリプレイ
fn replay_events(events: &[AccountEvent], account: Account) -> Trampoline<Account> {
    if events.is_empty() {
        Trampoline::done(account)
    } else {
        let updated = account.apply(&events[0]);
        Trampoline::suspend(move || replay_events(&events[1..], updated))
    }
}

// スタックオーバーフローなしで実行
let account = replay_events(&events, Account::default()).run();
```

### 不変イベントストレージ

構造共有による効率的な不変イベントシーケンスのために [`PersistentList`](../../docs/external/readme/README.ja.md#persistentlist) を使用します。

```rust
use lambars::persistent::PersistentList;

// イベントは不変リストに保存される
let events: PersistentList<AccountEvent> = event_store.load_events(&account_id);

// 新しいイベントを追加すると新しいリストが作成される（元は変更されない）
let new_events = events.cons(new_event);

// 構造共有によりメモリオーバーヘッドは最小限
assert_eq!(events.len(), original_count);      // 元は変更されない
assert_eq!(new_events.len(), original_count + 1);
```

### do 記法による非同期ワークフロー合成

クリーンな非同期ワークフロー合成のために [`eff_async!`](../../docs/external/readme/README.ja.md#eff_async-マクロ) と [`ExceptT`](../../docs/external/readme/README.ja.md#モナド変換子) を使用します。

**従来のスタイル（? 演算子）：**

```rust
pub async fn deposit_handler(...) -> Result<Json<Response>, ApiError> {
    let events = deps.event_store()
        .load_events(&id)
        .run_async()
        .await
        .map_err(|e| event_store_error(&e))?;

    let account = Account::from_events(&events)
        .ok_or_else(|| not_found_error())?;

    let event = either_to_result(deposit(&command, &account, timestamp))
        .map_err(|e| domain_error(&e))?;

    deps.event_store()
        .append_events(&id, vec![event.clone()])
        .run_async()
        .await
        .map_err(|e| event_store_error(&e))?;

    Ok(Json(response))
}
```

**eff_async! スタイル（do 記法）：**

```rust
use lambars::eff_async;
use lambars::effect::ExceptT;

// WorkflowResult は ExceptT をラップしてエラーハンドリングをクリーンに
type WorkflowResult<A> = ExceptT<ApiError, AsyncIO<Result<A, ApiError>>>;

async fn execute_workflow(...) -> Result<MoneyDeposited, ApiError> {
    let workflow: WorkflowResult<MoneyDeposited> = eff_async! {
        // 各ステップは自動的にエラーを伝播
        event <= from_result(deposit(&command, &account, timestamp).map_err(domain_error));
        _ <= lift_async_result(event_store.append_events(&id, vec![event.clone()]), event_store_error);
        _ <= lift_async_result(read_model.invalidate(&id), cache_error);
        pure_async(event)
    };

    workflow.run_async_io().run_async().await
}
```

### エラー蓄積による並列バリデーション

最初のエラーで失敗するのではなく、すべてのバリデーションエラーを収集するために `Validated`（Applicative ベースのバリデーション）を使用します。

```rust
use bank::domain::validation::Validated;

// 各バリデータは Validated<Vec<Error>, T> を返す
let result: Validated<Vec<ValidationError>, Account> = Validated::map3(
    validate_owner_name(name),      // Invalid(vec![NameTooLong]) を返す可能性
    validate_initial_balance(balance), // Invalid(vec![NegativeBalance]) を返す可能性
    validate_currency(currency),    // Invalid(vec![UnsupportedCurrency]) を返す可能性
    |name, balance, currency| Account::new(name, balance, currency)
);

// すべてのエラーが蓄積される（最初のエラーだけでなく）
match result {
    Validated::Valid(account) => Ok(account),
    Validated::Invalid(errors) => {
        // errors にはすべてのバリデーション失敗が含まれる
        Err(ValidationErrors(errors))
    }
}
```

### Writer モナドによる監査ログ

計算結果と一緒に監査ログを蓄積するために [`Writer`](../../docs/external/readme/README.ja.md#writer-モナド) を使用します。

```rust
use lambars::effect::Writer;
use bank::domain::audit::AuditEntry;

// 結果と監査証跡の両方を生成するワークフロー
fn audited_deposit(
    command: &DepositCommand,
    account: &Account,
    timestamp: Timestamp,
) -> Writer<Vec<AuditEntry>, MoneyDeposited> {
    Writer::tell(vec![AuditEntry::operation_started("deposit", timestamp)])
        .then(Writer::pure(deposit_logic(command, account)))
        .flat_map(|event| {
            Writer::tell(vec![AuditEntry::operation_completed("deposit", &event)])
                .then(Writer::pure(event))
        })
}

// 実行して結果とログの両方を取得
let (event, audit_logs) = audited_deposit(&command, &account, timestamp).run();

// audit_logs には完全な監査証跡が含まれる：
// - 操作開始時刻
// - アクター情報
// - 操作結果
// - 変更前/変更後の状態
```

## API エンドポイント

| メソッド | パス | 説明 |
|----------|------|------|
| `POST` | `/accounts` | 新しいアカウントを作成 |
| `GET` | `/accounts/:id` | アカウント情報を取得 |
| `GET` | `/accounts/:id/balance` | アカウント残高を取得 |
| `POST` | `/accounts/:id/deposit` | 入金（従来のスタイル） |
| `POST` | `/accounts/:id/deposit-eff` | 入金（eff_async! スタイル） |
| `POST` | `/accounts/:id/withdraw` | 出金（従来のスタイル） |
| `POST` | `/accounts/:id/withdraw-eff` | 出金（eff_async! スタイル） |
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
