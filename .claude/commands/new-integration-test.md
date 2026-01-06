# 新規Integrationテストシナリオ作成

以下のテンプレートに基づいてIntegrationテストシナリオファイルを作成してください。

## ファイルパス

`docs/internal/integration_tests/YYYYMMDD_HHMM_<name>.yaml`

- YYYYMMDD_HHMM: 現在のタイムスタンプ
- name: テストシナリオ名（snake_case）

## テンプレート

```yaml
# <タイトル> Integration テストシナリオ
#
# 概要:
#   <テストシナリオの概要を1-2行で記述>
#
# 対象モジュール:
#   - <モジュール1>
#   - <モジュール2>
#
# 関連要件:
#   - <関連する要件定義ファイル>

version: "1.0.0"
name: "<テストシナリオ名>"
description: |
  <テストシナリオの詳細な説明>
  複数行で記述可能。

# テスト対象
target:
  modules:
    - name: "<モジュール名>"
      path: "src/<path>"
      description: "<モジュールの説明>"
  features:
    - "<必要なfeatureフラグ>"

# 前提条件
prerequisites:
  - "<前提条件1>"
  - "<前提条件2>"

# テストシナリオ一覧
scenarios:
  # ======================================================================
  # 1. <カテゴリ名>
  # ======================================================================
  - id: <scenario_id>
    name: "<シナリオ名>"
    description: |
      <シナリオの詳細な説明>

    # テストの種類: unit, integration, property, e2e
    type: integration

    # 優先度: critical, high, medium, low
    priority: high

    # セットアップ
    setup:
      description: |
        <セットアップの説明>
      code: |
        // セットアップコード
        let setup_data = ...;

    # テストステップ
    steps:
      - step: 1
        action: "<アクションの説明>"
        input: |
          <入力データまたはコード>
        expected: |
          <期待される結果>

      - step: 2
        action: "<アクションの説明>"
        input: |
          <入力データまたはコード>
        expected: |
          <期待される結果>

    # アサーション
    assertions:
      - description: "<アサーションの説明>"
        code: |
          assert_eq!(actual, expected);

    # クリーンアップ（必要な場合）
    cleanup:
      description: |
        <クリーンアップの説明>
      code: |
        // クリーンアップコード

    # テストコードの例
    test_code: |
      #[test]
      fn test_<scenario_name>() {
          // Setup
          <setup_code>

          // Execute
          <execution_code>

          // Assert
          <assertion_code>
      }

# rstest を使用したパラメータ化テスト
parameterized_tests:
  - id: <param_test_id>
    name: "<パラメータ化テスト名>"
    description: |
      <テストの説明>

    # パラメータ定義
    parameters:
      - name: "<パラメータ名>"
        type: "<型>"
        description: "<パラメータの説明>"

    # テストケース
    cases:
      - name: "<ケース名>"
        values:
          <param1>: <value1>
          <param2>: <value2>
        expected: <expected_value>

      - name: "<ケース名>"
        values:
          <param1>: <value1>
          <param2>: <value2>
        expected: <expected_value>

    # rstest コード例
    test_code: |
      #[rstest]
      #[case(<value1>, <value2>, <expected>)]
      #[case(<value1>, <value2>, <expected>)]
      fn test_<name>(
          #[case] input1: Type1,
          #[case] input2: Type2,
          #[case] expected: ExpectedType,
      ) {
          let actual = function(input1, input2);
          assert_eq!(actual, expected);
      }

# プロパティベーステスト（オプション）
property_tests:
  - id: <prop_test_id>
    name: "<プロパティテスト名>"
    description: |
      <テストの説明>

    # テスト対象のプロパティ/法則
    properties:
      - name: "<プロパティ名>"
        description: |
          <プロパティの説明>
        equation: "<等式表現>"

    # proptest コード例
    test_code: |
      proptest! {
          #[test]
          fn prop_<name>(input in any::<Type>()) {
              // Property assertion
              prop_assert!(<property_holds>);
          }
      }

# エッジケース
edge_cases:
  - id: <edge_case_id>
    name: "<エッジケース名>"
    description: |
      <エッジケースの説明>
    input: "<入力>"
    expected_behavior: |
      <期待される振る舞い>
    test_code: |
      #[test]
      fn test_edge_case_<name>() {
          // Edge case test
      }

# エラーケース
error_cases:
  - id: <error_case_id>
    name: "<エラーケース名>"
    description: |
      <エラーケースの説明>
    input: "<入力>"
    expected_error: "<期待されるエラー>"
    test_code: |
      #[test]
      fn test_error_<name>() {
          let result = function(invalid_input);
          assert!(result.is_err());
      }

# 実行方法
execution:
  command: "cargo test --features <features> -- <test_filter>"
  coverage_command: "cargo llvm-cov --features <features> -- <test_filter>"

# メタデータ
metadata:
  author: "<作成者>"
  created_at: "<作成日>"
  updated_at: "<更新日>"
  related_requirements:
    - "<関連要件ID>"
  related_issues:
    - "<関連Issue番号>"
```

## 必須フィールド

- `version`: セマンティックバージョン
- `name`: テストシナリオ名
- `description`: テストシナリオの説明
- `target`: テスト対象のモジュール
- `scenarios`: テストシナリオのリスト
  - `id`: 一意の識別子
  - `name`: シナリオ名
  - `description`: シナリオの説明
  - `steps`: テストステップ
  - `assertions`: アサーション

## オプションフィールド

- `parameterized_tests`: rstest を使用したパラメータ化テスト
- `property_tests`: proptest を使用したプロパティベーステスト
- `edge_cases`: エッジケース
- `error_cases`: エラーケース

## 引数

$ARGUMENTS
