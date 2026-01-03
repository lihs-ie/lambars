# lambars

Rust で関数型プログラミングを行うために標準で提供していない関数型プログラミングの API を作成する

## ディレクトリ構成

```
src/
├── typeclass/      # 関数型プログラミングの型クラス階層
├── compose/        # 関数合成ユーティリティ
├── control/        # 制御構造（遅延評価、スタック安全な再帰、継続など）
├── persistent/     # 永続データ構造
├── optics/         # 深くネストした不変データ構造を型安全に操作するためのユーティリティ
└── effect/         # 副作用を型レベルで追跡・制御するためのシステム
lambars-derive/     # proc-macro クレート（derive マクロ）
docs/               # 仕様・設計ドキュメント（開発者向け）
├── internal/
│   ├── plans/          # タスクの実行計画
│   ├── requirements/   # タスクの要件定義
│   ├── future-work/    # 実装困難・後回しにしたもの
│   ├── done/           # 実装完了済みの記録
│   └── deprecated/     # 廃止された設計
└── ...
samples/            # サンプルプロジェクト
benches/            # ベンチマーク
CHANGELOG.md        # 更新履歴
```

## 開発コマンド

タスク一覧は `just --list` で確認できる。

### ビルド

```bash
# コンパイルチェック（高速）
cargo check

# 全 feature でビルド
cargo build

# リリースビルド
cargo build --release

# feature なしでビルド
cargo build --no-default-features

# 特定 feature のみ
cargo build --features "typeclass,compose"
```

### テスト

```bash
# 全テスト実行
cargo test

# feature なしでテスト
cargo test --no-default-features

# 特定のテストファイル
cargo test --test for_macro_tests

# 特定のテスト関数
cargo test test_function_name

# テスト出力を表示
cargo test -- --nocapture

# 並列数を制限
cargo test -- --test-threads=1

# 個別 feature 指定例
cargo test --features "typeclass,persistent"
```

### Lint・フォーマット

```bash
# clippy（lint）
cargo clippy

# clippy 全警告
cargo clippy -- -W clippy::all

# フォーマット
cargo fmt

# フォーマットチェック（CI用）
cargo fmt -- --check
```

### ドキュメント

```bash
# ドキュメント生成
cargo doc

# ドキュメント生成＆ブラウザで開く
cargo doc --open

# 警告をエラーとして扱う（CI用）
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

### カバレッジ

```bash
# llvm-cov でカバレッジ
cargo llvm-cov

# HTML レポート生成
cargo llvm-cov --html

# レポートを開く
open target/llvm-cov/html/index.html
```

### ベンチマーク

```bash
# 全ベンチマーク
cargo bench

# 特定のベンチマーク
cargo bench --bench for_macro_bench
```

### その他

```bash
# マクロ展開を確認（デバッグ用）
cargo expand --lib --tests

# act で GitHub Actions をローカル実行
act -j check

# ビルド成果物削除
cargo clean
```

## 開発ポリシー

- `cargo check` を通過すること
- `cargo clippy` を通過すること
- `cargo fmt` を実行すること
- テストカバレッジ 100%を目指すこと

### 技術要件

- Rust: 1.92.0
- edition: 2024
- testing: rstest

## 実装手順

1. サブエージェント: functional-programming-specialist を起動し要件定義を作成する
   1. 課題を解決するための方法をステップバイステップで考え、要件定義を yaml ファイルで `docs/internal/requirements/` に作成する
   2. rust-implementation-reviewer を起動し要件定義に対して実装計画を yaml ファイルで `docs/internal/plans/` に作成する
   3. functional-programming-specialist は実装計画が要件定義と異なる点がなくなるまでレビュー指摘を行う
   4. レビュー指摘がなくなるまで繰り返す（軽微な指摘も全て解決すること）
2. サブエージェント: rust-implementation-specialist を起動し実装計画に則って TDD で実装を行う
3. rust-implementation-reviewer を起動して実装のレビューを行う
   1. 略語を使用していないこと
   2. 差分の対象となるテストのみを実行し失敗していないこと
   3. 差分の対象となるテストのカバレッジ 100%であること
   4. レビュー指摘がなくなるまで修正とレビューを繰り返す（軽微な指摘も全て解決すること）
4. functional-programming-specialist を起動し要件定義の観点から実装をレビューする
5. `docs/roadmap.yaml` を更新する
6. 実装上困難だと判断した場合は `docs/internal/future-work/` に将来の拡張案としてファイルを書き出すこと
   1. 書き出しが完了した場合は github mcp を使って issue を作成すること

## コミットメッセージ

[Conventional Commits](https://www.conventionalcommits.org/) に従う。CHANGELOG.md は `cargo git-cliff` で自動生成される。

```
<type>(<scope>): <description>
```

| type       | 用途               |
| ---------- | ------------------ |
| `feat`     | 新機能             |
| `fix`      | バグ修正           |
| `docs`     | ドキュメント       |
| `refactor` | リファクタリング   |
| `perf`     | パフォーマンス改善 |
| `test`     | テスト追加・修正   |
| `chore`    | ビルド・CI など    |
| `deps`     | 依存関係の更新     |

| scope        |
| ------------ |
| `typeclass`  |
| `compose`    |
| `control`    |
| `persistent` |
| `optics`     |
| `effect`     |
| `ci`         |
| `docs`       |
| その他       |

```bash
# 例
feat(typeclass): add xxx support
docs: update README
```

## テストポリシー

テストピラミッドに基づき、可能な限り低レイヤーでテストする。

| レイヤー    | ディレクトリ  | 対象         |
| ----------- | ------------- | ------------ |
| Unit        | `src/**/*.rs` | 純粋ロジック |
| Integration | `tests/`      | 統合テスト   |
