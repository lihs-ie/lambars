# 統計結果フォーマット定義

## 概要

ベンチマーク実行時に生成される `stats.json` の形式と使用方法を定義します。このフォーマットは、複数回の測定結果から統計的信頼性を担保した結果を記録するために使用されます。

## stats.json スキーマ

### 全体構造

```json
{
  "<metric_name>": {
    "mean": <number>,
    "stddev": <number>,
    "stderr": <number>,
    "min": <number>,
    "max": <number>,
    "samples": <integer>,
    "confidence_interval_95": [<number>, <number>],
    "ci_width_ratio": <number>
  },
  ...
}
```

### フィールド定義

各メトリック（RPS, latency, P99 など）は以下のフィールドを持ちます。

| フィールド | 型 | 説明 |
|-----------|------|------|
| `mean` | number | 平均値。全サンプルの算術平均 |
| `stddev` | number | 標準偏差（Standard Deviation）。データのばらつきを示す |
| `stderr` | number | 標準誤差（Standard Error）。平均値の推定精度を示す。`stddev / sqrt(samples)` で計算 |
| `min` | number | 最小値。全サンプル中の最小値 |
| `max` | number | 最大値。全サンプル中の最大値 |
| `samples` | integer | サンプル数。測定回数 |
| `confidence_interval_95` | array | 95%信頼区間。t分布を使用して計算。`[下限, 上限]` の形式 |
| `ci_width_ratio` | number | CI幅比率。`(上限 - 下限) / mean` で計算。収束判定に使用（< 0.1 で収束とみなす）。注: mean が 0 に近い場合は除算を行わず、代わりに CI幅の絶対値で判定 |

### メトリック名の規約

| メトリック名 | 説明 | 単位 |
|-------------|------|------|
| `rps` | Requests per Second。1秒あたりのリクエスト処理数 | req/s |
| `latency_mean_ms` | 平均レイテンシ | ミリ秒 |
| `latency_stdev_ms` | レイテンシ標準偏差 | ミリ秒 |
| `p50_latency_ms` | P50 レイテンシ（中央値） | ミリ秒 |
| `p75_latency_ms` | P75 レイテンシ | ミリ秒 |
| `p90_latency_ms` | P90 レイテンシ | ミリ秒 |
| `p95_latency_ms` | P95 レイテンシ | ミリ秒 |
| `p99_latency_ms` | P99 レイテンシ | ミリ秒 |
| `error_rate` | エラー率（非2xxレスポンスの割合） | 0.0 〜 1.0 |
| `error_2xx_rate` | 2xx レスポンスの割合 | 0.0 〜 1.0 |
| `error_409_rate` | 409 Conflict の割合 | 0.0 〜 1.0 |
| `error_422_rate` | 422 Validation Error の割合 | 0.0 〜 1.0 |
| `error_5xx_rate` | 5xx Server Error の割合 | 0.0 〜 1.0 |

## 使用例

### 例1: 収束・未収束が混在する測定結果

```json
{
  "rps": {
    "mean": 42.64,
    "stddev": 0.8,
    "stderr": 0.462,
    "min": 41.8,
    "max": 43.4,
    "samples": 3,
    "confidence_interval_95": [40.65, 44.63],
    "ci_width_ratio": 0.093
  },
  "p99_latency_ms": {
    "mean": 27120,
    "stddev": 900,
    "stderr": 520,
    "min": 26150,
    "max": 27950,
    "samples": 3,
    "confidence_interval_95": [24883, 29357],
    "ci_width_ratio": 0.165
  }
}
```

この例では（n=3 で t(0.975, 2) = 4.303 を使用）：
- RPS は平均 42.64 req/s、95%信頼区間は [40.65, 44.63]
  - CI幅: 4.303 × 0.462 ≈ 1.99 → [42.64 - 1.99, 42.64 + 1.99]
  - CI幅比率 0.093 (9.3%) で収束判定基準（< 10%）を満たす
- P99 レイテンシは平均 27120 ms、95%信頼区間は [24883, 29357] ms
  - CI幅: 4.303 × 520 ≈ 2237 → [27120 - 2237, 27120 + 2237]
  - CI幅比率 0.165 (16.5%) で未収束（追加測定が必要）

注: 収束判定はメトリック単位で行います。全メトリックが収束するまで反復を継続します。

### 例2: 収束していない測定結果

```json
{
  "rps": {
    "mean": 42.5,
    "stddev": 5.8,
    "stderr": 3.35,
    "min": 35.2,
    "max": 48.9,
    "samples": 3,
    "confidence_interval_95": [28.1, 56.9],
    "ci_width_ratio": 0.678
  }
}
```

この例では（n=3 で t(0.975, 2) = 4.303 を使用）：
- CI幅: 4.303 × 3.35 ≈ 14.4 → [42.5 - 14.4, 42.5 + 14.4]
- CI幅比率 0.678 (67.8%) で収束判定基準（< 10%）を大きく超えている
- 追加測定が必要（最大10回まで反復）

## 収束判定基準

測定の信頼性を担保するため、以下の基準で収束を判定します。

### 収束条件

```
ci_width_ratio < 0.1
```

つまり、95%信頼区間の幅が平均値の10%未満であれば収束とみなします。

**注**: 平均値が 0 に近い場合（例: エラー率が 0）は、`ci_width_ratio` は CI幅の絶対値を表します。この場合、絶対値 < 0.1 で収束とみなします（例: エラー率の CI幅が 0.05 未満）。

### 反復測定ルール

1. **最小回数**: 3回（統計的信頼性の最低要件）
2. **収束チェック**: 3回目以降、毎回収束条件をチェック
3. **最大回数**: 10回（収束しない場合は警告を出して終了）
4. **全測定タイムアウト**: 30分以内
   - 各測定30秒 × 10回 = 5分（測定時間）
   - 統計計算、結果記録、収束判定などの処理時間を含めて30分を上限とする

### 収束しない場合

最大10回の測定でも収束しない場合、以下の原因が考えられます。

- **環境ノイズ**: バックグラウンドプロセス、CPU周波数変動など
- **測定対象の変動**: キャッシュヒット率の変動、ネットワーク遅延など
- **測定時間不足**: 30秒の測定では安定しない場合がある

対策：
- [環境ノイズ対策ガイド](./environment_setup.md) を参照
- 測定時間を延長（`-d 60s` など）
- self-hosted runner の使用を検討

## スキーマ検証方法

### JSON スキーマによる検証（推奨）

```bash
# 注: stats_schema.json は今後実装予定
# 現時点では以下のような検証スクリプトを使用

cd benches/api/benchmarks
python3 -c "
import json
import jsonschema

# 将来の実装: schema/stats_schema.json
# with open('schema/stats_schema.json') as f:
#     schema = json.load(f)

with open('results/my_benchmark/stats.json') as f:
    data = json.load(f)

# 基本的なフィールド存在チェック
for metric, values in data.items():
    required = ['mean', 'stddev', 'stderr', 'min', 'max', 'samples', 'confidence_interval_95', 'ci_width_ratio']
    for field in required:
        assert field in values, f'Missing {field} in {metric}'

print('✓ 基本的な検証成功')
"
```

### シェルスクリプトによる検証

```bash
# 注: validate_stats.sh は今後実装予定
# 現時点では上記の Python スクリプトを使用してください

# 必須フィールドの存在確認（将来の実装）
# cd benches/api/benchmarks
# ./scripts/validate_stats.sh results/my_benchmark/stats.json
```

### Python スクリプトによる検証

```python
import json

def validate_stats(stats_file):
    with open(stats_file) as f:
        data = json.load(f)

    for metric_name, metric_data in data.items():
        required_fields = [
            'mean', 'stddev', 'stderr',
            'min', 'max', 'samples',
            'confidence_interval_95', 'ci_width_ratio'
        ]

        for field in required_fields:
            assert field in metric_data, f"{metric_name}: missing field '{field}'"

        # CI の妥当性チェック
        ci = metric_data['confidence_interval_95']
        assert len(ci) == 2, f"{metric_name}: CI must have 2 elements"
        assert ci[0] <= metric_data['mean'] <= ci[1], \
            f"{metric_name}: mean must be within CI"

        # CI幅比率の妥当性チェック
        ci_width = ci[1] - ci[0]
        mean = metric_data['mean']

        # mean が 0 に近い場合は CI幅の絶対値を使用
        if abs(mean) < 1e-6:
            expected_ratio = ci_width
        else:
            expected_ratio = ci_width / mean

        actual_ratio = metric_data['ci_width_ratio']
        assert abs(expected_ratio - actual_ratio) < 0.01, \
            f"{metric_name}: ci_width_ratio mismatch"

    print("✓ 全ての検証をパス")

validate_stats('results/my_benchmark/stats.json')
```

## 統計計算の詳細

### Welch の t 検定による有意性判定

測定結果の統計的有意性を判定するため、Welch の t 検定を使用します。これは、2つのサンプル間に統計的に有意な差があるかを判定する手法です。

```python
from scipy import stats
from typing import List, Tuple

def is_significant_change(
    before_samples: List[float],
    after_samples: List[float],
    alpha: float = 0.05
) -> Tuple[bool, float]:
    """
    Welch の t 検定を使用して有意性を判定。

    Args:
        before_samples: 変更前の測定値リスト
        after_samples: 変更後の測定値リスト
        alpha: 有意水準（デフォルト 0.05）

    Returns:
        (is_significant, p_value): 有意かどうかと p 値のタプル
    """
    # Welch の t 検定（等分散を仮定しない）
    statistic, p_value = stats.ttest_ind(
        before_samples,
        after_samples,
        equal_var=False  # Welch's t-test
    )

    is_significant = p_value < alpha
    return (is_significant, p_value)
```

**判定基準**:
- p値 < 0.05: 統計的に有意な変化（真の変化）
- p値 >= 0.05: 統計的に有意でない（測定ブレの可能性）

### t分布による信頼区間

サンプル数が少ない場合（n < 30）、正規分布ではなく t分布を使用します。

```python
from scipy import stats
import numpy as np

def calculate_confidence_interval(samples, confidence=0.95):
    """
    t分布を使用した95%信頼区間の計算

    Args:
        samples: 測定値のリスト
        confidence: 信頼水準（デフォルト: 0.95）

    Returns:
        (lower, upper): 信頼区間の下限と上限
    """
    n = len(samples)
    mean = np.mean(samples)
    stderr = stats.sem(samples)  # 標準誤差
    h = stderr * stats.t.ppf((1 + confidence) / 2, n - 1)  # t分布の臨界値
    return (mean - h, mean + h)
```

### CI幅比率の計算

```python
def calculate_ci_width_ratio(samples):
    """
    CI幅比率の計算

    Args:
        samples: 測定値のリスト

    Returns:
        float: CI幅 / mean（mean が 0 に近い場合は CI幅の絶対値）
    """
    lower, upper = calculate_confidence_interval(samples)
    mean = np.mean(samples)
    ci_width = upper - lower

    # mean が 0 に近い場合（例: エラー率が 0）は CI幅の絶対値で判定
    if abs(mean) < 1e-6:
        return ci_width

    return ci_width / mean
```

## CI/CD での使用

### GitHub Actions での例

```yaml
# 注: 以下のスクリプトは今後実装予定

- name: Run benchmark with statistical reliability
  run: |
    cd benches/api/benchmarks
    # 将来の実装: ./scripts/run_benchmark_with_stats.sh \
    #   --scenario scenarios/tasks_eff.yaml \
    #   --output results/${{ github.sha }}/stats.json

- name: Check convergence
  run: |
    cd benches/api/benchmarks
    # 将来の実装: python3 scripts/check_convergence.py \
    #   results/${{ github.sha }}/stats.json \
    #   --threshold 0.1

- name: Upload stats artifact
  uses: actions/upload-artifact@v3
  with:
    name: benchmark-stats
    path: benches/api/benchmarks/results/${{ github.sha }}/stats.json
```

## 参考資料

- [ベンチマーク網羅性改善 要件定義](../../../docs/internal/requirements/20260201_1300_benchmark_coverage_improvement.yaml)
- [環境ノイズ対策ガイド](./environment_setup.md)
- [tasks_bulk スケール別ベンチマークガイド](./tasks_bulk_scale.md)
- [scipy.stats.t documentation](https://docs.scipy.org/doc/scipy/reference/generated/scipy.stats.t.html)
- [Student's t-distribution (Wikipedia)](https://en.wikipedia.org/wiki/Student%27s_t-distribution)
