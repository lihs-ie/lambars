# 新規要件定義作成

以下のテンプレートに基づいて要件定義ファイルを作成してください。

## ファイルパス

`docs/internal/requirements/YYYYMMDD_HHMM_<name>.yaml`

- YYYYMMDD_HHMM: 現在のタイムスタンプ
- name: 機能名（snake_case）

## テンプレート

```yaml
# <タイトル> 要件定義
#
# 概要:
#   <機能の概要を1-2行で記述>
#
# 設計方針:
#   1. <方針1>
#   2. <方針2>
#   3. <方針3>
#
# 参照:
#   - <関連ドキュメント>
#   - <参考資料>

version: "1.0.0"
name: "<機能名>"
description: |
  <機能の詳細な説明>
  複数行で記述可能。

# 背景・動機
background:
  problem: |
    <解決すべき問題>
  motivation: |
    <この機能を実装する動機>
  prior_art:
    - name: "<参考となる既存実装>"
      description: "<説明>"

# 要件一覧
requirements:
  # ======================================================================
  # 1. <カテゴリ名>
  # ======================================================================
  - id: <requirement_id>
    name: "<要件名>"
    description: |
      <要件の詳細な説明>

    # 法則（該当する場合）
    laws:
      - name: "<法則名>"
        description: |
          <法則の説明>
        equation: "<等式表現>"
        property_test: |
          <プロパティテストのコード>

    # メソッド定義
    methods:
      - name: "<メソッド名>"
        signature: "<シグネチャ>"
        description: |
          <メソッドの説明>
        examples:
          - description: "<例の説明>"
            code: |
              <コード例>

    # 実装対象の型
    implementations:
      - type: "<型名>"
        description: |
          <この型に対する実装の説明>

# 非機能要件
non_functional_requirements:
  performance:
    - "<パフォーマンス要件>"
  compatibility:
    - "<互換性要件>"
  testing:
    - "<テスト要件>"

# 将来の拡張
future_extensions:
  - id: "<拡張ID>"
    name: "<拡張名>"
    description: |
      <将来の拡張の説明>
    rationale: |
      <現時点で実装しない理由>
```

## 必須フィールド

- `version`: セマンティックバージョン
- `name`: 機能名
- `description`: 機能の説明
- `requirements`: 要件のリスト
  - `id`: 一意の識別子
  - `name`: 要件名
  - `description`: 要件の説明

## 引数

$ARGUMENTS
