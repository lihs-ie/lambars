# 新規実装計画作成

以下のテンプレートに基づいて実装計画ファイルを作成してください。

## ファイルパス

`docs/internal/plans/YYYYMMDD_HHMM_<name>.yaml`

- YYYYMMDD_HHMM: 現在のタイムスタンプ
- name: 機能名（snake_case）

## テンプレート

```yaml
# <タイトル> 実装計画
#
# 要件定義: docs/internal/requirements/YYYYMMDD_HHMM_<name>.yaml
# 技術要件:
#   - Rust: 1.92.0
#   - edition: 2024
#   - testing: rstest, proptest
#
# 実装方針:
#   1. TDD（テスト駆動開発）でテストを先に書く
#   2. <方針2>
#   3. <方針3>

version: "1.0.0"
name: "<機能名> Implementation Plan"
requirement_file: "docs/internal/requirements/YYYYMMDD_HHMM_<name>.yaml"

# 実装順序の概要
implementation_order:
  - step: 1
    name: "<ステップ名>"
    items:
      - <実装項目1>
      - <実装項目2>
  - step: 2
    name: "<ステップ名>"
    items:
      - <実装項目3>
      - <実装項目4>

# 実装計画詳細
implementation_plan:
  # ============================================================================
  # 1. <セクション名>
  # ============================================================================
  - id: impl_<identifier>
    requirement_id: <対応する要件ID>
    name: "<実装タスク名>"
    priority: 1
    description: |
      <実装タスクの詳細な説明>

    files:
      - path: src/<module>/<file>.rs
        description: |
          <このファイルで実装する内容の説明>

    implementation_steps:
      - step: 1
        description: |
          <ステップの説明>
        code_outline: |
          /// ドキュメントコメント
          ///
          /// # Examples
          ///
          /// ```
          /// // コード例
          /// ```
          pub trait/fn/struct <Name> {
              // 実装の概要
          }

      - step: 2
        description: |
          <次のステップの説明>
        code_outline: |
          impl<T> Trait for Type {
              // 実装の概要
          }

    tests:
      - name: test_<name>
        description: <テストの説明>
        test_type: unit  # unit, integration, property
        code_outline: |
          #[cfg(test)]
          mod tests {
              use super::*;

              #[test]
              fn test_name() {
                  // テストコード
              }
          }

      - name: prop_<name>
        description: <プロパティテストの説明>
        test_type: property
        code_outline: |
          proptest! {
              #[test]
              fn prop_name(input in any::<Type>()) {
                  // プロパティ検証
              }
          }

    dependencies:
      - <依存する実装タスクのID>

# テスト戦略
test_strategy:
  unit_tests:
    location: src/<module>/<file>.rs
    description: |
      <ユニットテストの方針>

  integration_tests:
    location: tests/<name>_tests.rs
    description: |
      <統合テストの方針>

  property_tests:
    location: tests/<name>_laws.rs
    description: |
      <プロパティテストの方針>

# 完了条件
acceptance_criteria:
  - <完了条件1>
  - <完了条件2>
  - cargo check が通過すること
  - cargo clippy が通過すること
  - cargo test が通過すること
  - カバレッジ 100% であること
```

## 必須フィールド

- `version`: セマンティックバージョン
- `name`: 実装計画名
- `requirement_file`: 対応する要件定義ファイルのパス
- `implementation_order`: 実装順序の概要
- `implementation_plan`: 詳細な実装計画
  - `id`: 一意の識別子
  - `name`: タスク名
  - `files`: 対象ファイル
  - `implementation_steps`: 実装ステップ
  - `tests`: テスト計画

## 引数

$ARGUMENTS
