# 新規 Issue 作成

実装が困難な項目や将来の拡張案を Issue としてファイルに記録してください。

## ファイルパス

`docs/internal/issues/YYYYMMDD_HHMM_<name>.yaml`

- YYYYMMDD_HHMM: 現在のタイムスタンプ
- name: Issue 名（snake_case）

## テンプレート

```yaml
# Issue: <タイトル>
# 実装時に発見された、将来対応が必要な項目

deferred_features:
  - id: <issue_id>
    name: "<Issue 名>"
    discovered_date: "YYYY-MM-DD"
    priority: high | medium | low
    category: enhancement | bug | research | refactoring
    labels:
      - "<ラベル1>"  # typeclass, compose, control, persistent, optics, effect など
      - "<ラベル2>"  # api, macro, traits, async, performance など

    # 問題の説明
    problem:
      summary: "<問題の要約（1行）>"
      details: |
        <問題の詳細な説明>
        - 何が問題なのか
        - なぜ発生するのか
        - どのような影響があるのか

      # Rust の制約による場合
      rust_limitation: |
        ```rust
        // なぜこのコードが書けないのかを説明
        ```

    # 解決策の提案
    solution:
      approach: "<推奨するアプローチの概要>"
      options:
        - name: "<オプション1>"
          description: |
            <オプションの説明>
          pros:
            - <メリット1>
            - <メリット2>
          cons:
            - <デメリット1>
            - <デメリット2>

        - name: "<オプション2>"
          description: |
            <オプションの説明>
          pros:
            - <メリット1>
          cons:
            - <デメリット1>

      recommended: "<推奨するオプション名>"
      estimated_complexity: low | medium | high | research

    # 参考情報
    references:
      - "<参考資料1>"
      - "<参考資料2>"

    # 関連する要件・実装
    related:
      requirement_id: "<関連する要件ID>"
      plan_id: "<関連する実装計画ID>"

    # GitHub Issue 情報（作成後に追記）
    github_issue:
      number: null  # Issue 番号
      url: null     # Issue URL
      created: false
```

## フィールド説明

### 必須フィールド

- `id`: 一意の識別子（例: `monad_flatten`）
- `name`: Issue の名前
- `discovered_date`: 発見日
- `priority`: 優先度（high/medium/low）
- `labels`: GitHub ラベルのリスト
- `problem.summary`: 問題の要約
- `solution.approach`: 解決策の概要

### オプションフィールド

- `category`: カテゴリ（enhancement/bug/research/refactoring）
- `rust_limitation`: Rust の制約による問題の場合のコード例
- `solution.options`: 複数の解決策オプション
- `references`: 参考資料
- `related`: 関連する要件・実装計画
- `github_issue`: GitHub Issue 情報

### 利用可能なラベル

**モジュール:**
- `typeclass`, `compose`, `control`, `persistent`, `optics`, `effect`

**カテゴリ:**
- `enhancement`, `bug`, `documentation`, `research`, `refactoring`

**特性:**
- `api`, `macro`, `traits`, `async`, `performance`, `safety`, `usability`

**優先度:**
- `priority: critical`, `priority: high`, `priority: medium`, `priority: low`

## GitHub Issue 作成

ファイル作成後、以下のコマンドで GitHub Issue を作成してください：

```
github mcp を使って Issue を作成
```

作成後、`github_issue` セクションを更新してください。

## 引数

$ARGUMENTS
