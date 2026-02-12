# DESIGN

## アーキテクチャ概要

tasks_bulk の merge 経路選択率を監視するため、プロファイル結果（stacks.folded）から with_arena / without_arena のサンプル数を抽出し、meta.json に merge_path_detail として追加する。CI で with_arena_ratio が閾値（0.90）を下回った場合に fail させることで、経路退行を自動検知する。

**現状の問題**:
- tasks_bulk の meta.json には merge 経路情報がない
- stacks.folded はプロファイル結果として生成されるが、メトリクス化されていない
- check_thresholds.sh には merge 経路のゲートがない

**解決策**:
run_benchmark.sh の generate_meta_json() で stacks.folded を解析し、merge_path_detail を meta.json に追加する。check_thresholds.sh で tasks_bulk 専用のゲートを追加し、with_arena_ratio が閾値未満の場合に exit code 3 を返す。

## 設計判断

### 決定1: stacks.folded から経路選択率を計算

**理由**:
- Rust 側でカウンタを実装するのは別タスク（IMPL-TBPA2-001）
- stacks.folded は既にプロファイル結果として生成されている
- awk で関数名パターンマッチングすれば with_arena / without_arena を区別可能

**実装**:

```bash
# stacks.folded から with_arena / without_arena の比率を計算
if [ -f "${RESULTS_DIR}/stacks.folded" ]; then
  total_samples=$(awk '{s+=$NF} END{print s}' "${RESULTS_DIR}/stacks.folded")
  with_arena=$(awk '$0~/merge_index_delta_add_only_owned_with_arena/{s+=$NF} END{print s+0}' "${RESULTS_DIR}/stacks.folded")
  without_arena=$(awk '$0~/merge_index_delta_add_only_owned[^_]/{s+=$NF} END{print s+0}' "${RESULTS_DIR}/stacks.folded")

  # with_arena_ratio を計算（ゼロ除算回避）
  if (( with_arena + without_arena > 0 )); then
    with_arena_ratio=$(awk -v a="${with_arena}" -v b="${without_arena}" 'BEGIN { printf "%.6f", a / (a + b) }')
  else
    with_arena_ratio="null"
  fi
fi
```

**代替案**:
1. **Rust 側でカウンタを実装**: より正確だが、別タスク（IMPL-TBPA2-001）で対応予定
2. **ログから抽出**: ログフォーマットが変わると壊れやすい

### 決定2: meta.json に merge_path_detail を追加

**理由**:
- meta.json は既にベンチマーク結果の一元管理に使われている
- jq で JSON 生成すれば型安全
- 他のメトリクス（error_rate, conflict_detail など）と同様のパターン

**実装**:

```bash
# generate_meta_json() の jq 呼び出しに --argjson で追加
local merge_path_detail_json="null"
if [[ -n "${with_arena_samples:-}" ]]; then
  merge_path_detail_json=$(jq -n \
    --argjson with_arena_samples "${with_arena_samples}" \
    --argjson without_arena_samples "${without_arena_samples}" \
    --argjson with_arena_ratio "${with_arena_ratio_json}" \
    '{
      "with_arena_samples": $with_arena_samples,
      "without_arena_samples": $without_arena_samples,
      "with_arena_ratio": $with_arena_ratio
    }')
fi

# meta.json の results セクションに追加
jq -n \
  --argjson merge_path_detail "${merge_path_detail_json}" \
  '{
    ...
    "results": {
      ...
      "merge_path_detail": $merge_path_detail
    }
  }'
```

**スキーマ**:
```json
{
  "results": {
    "merge_path_detail": {
      "with_arena_samples": <number>,
      "without_arena_samples": <number>,
      "with_arena_ratio": <float 0-1>
    }
  }
}
```

### 決定3: check_thresholds.sh に tasks_bulk 専用ゲートを追加

**理由**:
- 既存の check_thresholds.sh は p50/p90/p99/error_rate/conflict_rate をチェック
- tasks_bulk 専用の merge 経路選択率ゲートを追加することで、経路退行を自動検知
- with_arena_ratio が閾値（0.90）未満の場合に exit code 3 を返す

**実装完了 (IMPL-TBLR-002)**:
- merge_path_detail 欠落時は WARNING → **FAIL (exit 3)** に変更
- bulk_with_arena/bulk_without_arena 不完全時も **FAIL (exit 3)** に変更
- これにより CI で確実に検知可能になった

**実装**:

```bash
# check_thresholds.sh の末尾に tasks_bulk 専用ゲートを追加
if [[ "${SCENARIO}" == "tasks_bulk" ]]; then
    # merge_path_detail が存在するかチェック
    MERGE_PATH_DETAIL=$(jq -r '.results.merge_path_detail // empty' "${META_FILE}" 2>/dev/null || true)

    if [[ -z "${MERGE_PATH_DETAIL}" || "${MERGE_PATH_DETAIL}" == "null" ]]; then
        echo "WARNING: merge_path_detail not found in meta.json for tasks_bulk"
        echo "  This may indicate profiling was disabled or stacks.folded is missing"
    else
        WITH_ARENA_RATIO=$(jq -r '.results.merge_path_detail.with_arena_ratio // empty' "${META_FILE}" 2>/dev/null || true)

        if [[ -n "${WITH_ARENA_RATIO}" && "${WITH_ARENA_RATIO}" != "null" ]]; then
            # with_arena_ratio >= 0.90 をチェック
            if (( $(echo "${WITH_ARENA_RATIO} < 0.90" | bc -l) )); then
                echo "FAIL: merge_path_detail.with_arena_ratio=${WITH_ARENA_RATIO} is below threshold 0.90"
                echo "  This indicates bulk path is not using with_arena implementation"
                exit 3
            else
                echo "PASS: merge_path_detail.with_arena_ratio=${WITH_ARENA_RATIO} (>= 0.90)"
            fi
        fi
    fi
fi
```

**閾値の根拠**:
- Run 21886689088 では with_arena が 0.0037% で問題発生
- 目標: with_arena が 90% 以上（0.90）

### 決定4: テストスクリプトで meta.json を模擬

**理由**:
- check_thresholds.sh のゲートロジックをテストするため
- meta.json を手動作成して check_thresholds.sh を呼び出す
- Test 1: PASS (with_arena_ratio = 0.95)
- Test 2: FAIL (with_arena_ratio = 0.50)
- Test 3: WARNING (merge_path_detail 欠落)

**実装**:

```bash
# test_merge_path_gate.sh

test_pass() {
  # with_arena_ratio = 0.95 (PASS)
  jq -n '{
    "results": {
      "merge_path_detail": {
        "with_arena_samples": 950,
        "without_arena_samples": 50,
        "with_arena_ratio": 0.95
      }
    }
  }' > "${TMP_DIR}/meta.json"

  if check_thresholds.sh "${TMP_DIR}" tasks_bulk; then
    echo "PASS: Test 1 passed"
  else
    echo "FAIL: Test 1 should pass"
    exit 1
  fi
}

test_fail() {
  # with_arena_ratio = 0.50 (FAIL)
  jq -n '{
    "results": {
      "merge_path_detail": {
        "with_arena_samples": 500,
        "without_arena_samples": 500,
        "with_arena_ratio": 0.50
      }
    }
  }' > "${TMP_DIR}/meta.json"

  if check_thresholds.sh "${TMP_DIR}" tasks_bulk 2>&1 | grep -q "FAIL.*with_arena_ratio"; then
    echo "PASS: Test 2 failed as expected"
  else
    echo "FAIL: Test 2 should fail"
    exit 1
  fi
}

test_warning() {
  # merge_path_detail 欠落 (WARNING)
  jq -n '{
    "results": {}
  }' > "${TMP_DIR}/meta.json"

  if check_thresholds.sh "${TMP_DIR}" tasks_bulk 2>&1 | grep -q "WARNING.*merge_path_detail"; then
    echo "PASS: Test 3 warned as expected"
  else
    echo "FAIL: Test 3 should warn"
    exit 1
  fi
}
```

## 技術的詳細

### 変更ファイル一覧

1. **benches/api/benchmarks/run_benchmark.sh**
   - generate_meta_json() で stacks.folded を解析
   - merge_path_detail を meta.json に追加

2. **benches/api/benchmarks/check_thresholds.sh**
   - tasks_bulk 専用の merge 経路選択率ゲートを追加
   - with_arena_ratio が 0.90 未満の場合に exit code 3

3. **benches/api/benchmarks/scripts/test_merge_path_gate.sh**
   - PASS / FAIL / WARNING の3ケースをテスト

### データフロー

#### merge_path_detail 生成フロー

1. run_benchmark.sh が --profile モードで実行
2. perf record → perf script → stacks.folded 生成
3. generate_meta_json() で stacks.folded を解析
4. awk で merge_index_delta_add_only_owned_with_arena / merge_index_delta_add_only_owned のサンプル数を抽出
5. with_arena_ratio を計算（ゼロ除算回避）
6. jq で merge_path_detail を meta.json に追加

#### CI ゲートフロー

1. check_thresholds.sh が tasks_bulk シナリオを検出
2. meta.json から merge_path_detail を読み取り
3. with_arena_ratio が 0.90 未満の場合に exit code 3
4. GitHub Actions が exit code 3 を検知して CI を fail

### テスト戦略

#### 構文チェック

```bash
bash -n benches/api/benchmarks/run_benchmark.sh
bash -n benches/api/benchmarks/check_thresholds.sh
bash -n benches/api/benchmarks/scripts/test_merge_path_gate.sh
```

#### 機能テスト

1. **test_merge_path_gate.sh**:
   - Test 1: PASS (with_arena_ratio = 0.95)
   - Test 2: FAIL (with_arena_ratio = 0.50)
   - Test 3: WARNING (merge_path_detail 欠落)

### 決定4: 段階的回帰防止ゲート (IMPL-TBLR-003)

**理由**:
- merge_path_gate だけでは経路選択を保証するが、性能退行は検知できない
- 既存の p99/rps 閾値（80ms/N/A）は最適化後の目標値で、revert を検知する閾値としては厳しすぎる
- Run 21886689088 (p99=8320ms, rps推定=341.36) を revert ベースラインとして設定

**実装**:
- `check_bulk_regression_guard` 関数を追加
- P99 <= 9550ms, RPS >= 341.36 を revert 閾値として設定
- どちらか違反した場合は exit code 3 で fail
- check_merge_path_gate の後、既存閾値チェックの前に実行

**メリット**:
- 段階的なゲート（revert 防止 → 最適化目標）で適切な粒度の検知
- 既存閾値を変更せずに revert 検知を追加可能
- CI で性能退行を確実に検知できる

## 制約事項

- stacks.folded がない場合は merge_path_detail が null になる（プロファイル無効時）→ **IMPL-TBLR-002 で FAIL に変更**
- with_arena_samples + without_arena_samples が 0 の場合は with_arena_ratio が null → **IMPL-TBLR-002 で FAIL に変更**
- 関数名パターンマッチングで merge_index_delta_add_only_owned を検出（`[^_]` で _with_arena を除外）
- RPS は meta.json に含まれている必要がある（IMPL-TBLR-003）

## 将来の拡張性

- Rust 側でカウンタを実装すれば、プロファイル無効時でも経路選択率を取得可能
- 他のシナリオ（tasks_update など）でも同様のパターンで経路選択率を監視可能
- merge_path_detail に新しいフィールド（例: delta_with_arena_ratio）を追加可能
