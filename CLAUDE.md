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
├── internal/       # 内部設計
│   ├── plans/          # タスクの実行計画（YYYYMMDD_HHMM_名前.yaml）
│   ├── requirements/   # タスクの要件定義（YYYYMMDD_HHMM_名前.yaml）
│   ├── deprecated/     # 廃止された設計
│   ├── issues/         # 実装困難・後回しにしたものissue化したもの（YYYYMMDD_HHMM_名前.yaml）
│   └── done/           # 実装完了済みの記録
│       ├── plans/          # タスクの実行計画（YYYYMMDD_HHMM_名前.yaml）
│       ├── requirements/   # タスクの要件定義（YYYYMMDD_HHMM_名前.yaml）
│       └── issues/         # 実装困難・後回しにしたものissue化したもの（YYYYMMDD_HHMM_名前.yaml）
└── external/       # 外部設計（ライブラリ使用者のためのドキュメント）
    └── comparison/     # 他言語との API 対応表
        └── {language}/     # 言語名（Haskell, Scala, F# など）
samples/            # サンプルプロジェクト
benches/            # ベンチマーク
CHANGELOG.md        # 更新履歴
```

## 開発コマンド

タスク一覧は `cargo --list` で確認できる。

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

## レビュー体制

### 1. Codex レビュー

- **タイミング**: 各フェーズの設計・計画段階
- **目的**: 技術的な妥当性確認、ベストプラクティスの提案
- **対象**: 設計ドキュメント、実装計画、主要なコード変更
- **方法**: MCP `mcp__codex__codex` ツールを使用
- **確認事項**:
  - 既存コードとの整合性
  - パフォーマンスへの影響
  - セキュリティリスク
  - 関数型プログラミングとしてのアーキテクチャ妥当性
    - 参照透過性（Referential Transparency）
      - 同じ入力に対して常に同じ出力を返すか
      - 外部状態（グローバル変数、時刻、乱数など）に依存していないか
      - 関数呼び出しを「値」に置き換えても意味が変わらないか
    - 純粋関数（Pure Function）
      - 副作用（I/O、DB、ログ、状態変更）が含まれていないか
      - 「計算」と「実行」が分離されているか
      - テストが引数と戻り値だけで書けるか
    - 不変性（Immutability）
      - 引数や既存のデータを直接変更していないか
      - push, splice, ++, -- などの破壊的操作をしていないか
      - 新しい値を返す設計になっているか
    - 例外を制御フローに使っていないか
      - 失敗やエラーが型として表現されているか
      - Result, Either, Option などの値で表現できないか
    - 高階関数・コレクション操作が自然か
      - for / while の代わりに map / filter / reduce を使えているか
      - 「どう処理するか」より「何をしたいか」が読めるか
      - 処理の流れがパイプラインとして理解できるか

### 2. Sub-Agent レビュー

- **タイミング**: 必要に応じて（複雑な実装、重要な変更）
- **目的**: 特定領域の専門的レビュー
- **対象**: アーキテクチャ設計、コード品質、セキュリティ
- **方法**: Task ツールで専門エージェントを起動
  - `functional-programming-specialist`: アーキテクチャ・実装レビュー
- **確認事項**:
  - SOLID 原則の遵守
  - デザインパターンの適用
  - 長期的な保守性
  - 関数型プログラミングとしてのアーキテクチャ妥当性
    - 参照透過性（Referential Transparency）
      - 同じ入力に対して常に同じ出力を返すか
      - 外部状態（グローバル変数、時刻、乱数など）に依存していないか
      - 関数呼び出しを「値」に置き換えても意味が変わらないか
    - 純粋関数（Pure Function）
      - 副作用（I/O、DB、ログ、状態変更）が含まれていないか
      - 「計算」と「実行」が分離されているか
      - テストが引数と戻り値だけで書けるか
    - 不変性（Immutability）
      - 引数や既存のデータを直接変更していないか
      - push, splice, ++, -- などの破壊的操作をしていないか
      - 新しい値を返す設計になっているか
    - 例外を制御フローに使っていないか
      - 失敗やエラーが型として表現されているか
      - Result, Either, Option などの値で表現できないか
    - 高階関数・コレクション操作が自然か
      - for / while の代わりに map / filter / reduce を使えているか
      - 「どう処理するか」より「何をしたいか」が読めるか
      - 処理の流れがパイプラインとして理解できるか

## 実装手順

1. gh コマンド を使って PR を作成する
   1. issue を対応する場合は issue と PR を紐づける
2. サブエージェント: functional-programming-specialist を起動し要件定義を作成する
   1. `/new-requirement <機能名>` で要件定義テンプレートを取得する
   2. 課題を解決するための方法をステップバイステップで考え、要件定義を作成する
   3. codex mcp にレビューをさせる
   4. functional-programming-specialist は要件定義が関数型プログラミングのアーキテクチャとして妥当と判断するまでレビュー指摘を行う
   5. レビュー指摘がなくなるまで繰り返す（軽微な指摘も全て解決すること）
3. サブエージェント: rust-implementation-specialist を起動し要件定義に則って TDD で実装を行う
   1. テストは rstest をベースに作成すること
      1. 標準の test crate は使用しない
   2. ここまではテストが通ることまで確認できたらコミットする
4. rust-simplification-specialist を起動して今回変更・作成したコードの構造を簡素化する
5. codex mcp にレビューをさせる
   1. レビュー指摘がなくなるまで修正とレビューを繰り返す（軽微な指摘も全て解決すること）
6. functional-programming-specialist を起動し要件定義の観点から実装をレビューする
7. コミット前に以下の確認を実施する
   1. `cargo fmt` - コードフォーマット
   2. `cargo clippy --all-features --all-targets -- -D warnings` - lint チェック
   3. `cargo doc --no-deps` - ドキュメントビルド
   4. 全てパスしたらコミット
8. 実装した内容を README.md, docs/external/comparison に反映する必要があるか調査し、修正が必要な場合は変更を記載しコミットする
9. 対象の要件定義、issue 対応の場合は issue のファイルを `docs/internal/done/` に移動する
10. 実装上困難だと判断した場合は `/new-issue <Issue名>` で Issue ファイルを作成する
11. `docs/internal/issues/` に将来の拡張案として保存する
12. gh コマンド を使って GitHub Issue を作成し、ファイルの `github_issue` セクションを更新する

### スラッシュコマンド一覧

| コマンド                    | 説明                       |
| --------------------------- | -------------------------- |
| `/new-requirement <機能名>` | 要件定義テンプレートを取得 |
| `/new-plan <機能名>`        | 実装計画テンプレートを取得 |
| `/new-issue <Issue名>`      | Issue テンプレートを取得   |

## コミットメッセージ

[Conventional Commits](https://www.conventionalcommits.org/) に従う。CHANGELOG.md は `cargo git-cliff` で自動生成される。

```
<type>(<scope>): <description>
```

| type         | 用途                         |
| ------------ | ---------------------------- |
| `feat`       | 新機能                       |
| `fix`        | バグ修正                     |
| `docs`       | ドキュメント                 |
| `refactor`   | リファクタリング             |
| `perf`       | パフォーマンス改善           |
| `test`       | テスト追加・修正             |
| `chore`      | ビルド・CI など              |
| `deps`       | 依存関係の更新               |
| `modify`     | 開発支援ツールによる変更指示 |
| scope        |
| ------------ |
| `typeclass`  |
| `compose`    |
| `control`    |
| `persistent` |
| `optics`     |
| `effect`     |
| `derive`     |
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
