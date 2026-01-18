# Benchmarks

lambarsライブラリのベンチマークシステム。

## 概要

lambarsでは2種類のベンチマークを提供しています:

| 種類 | ツール | 用途 | CI統合 |
|------|--------|------|--------|
| **Iai-Callgrind** | iai-callgrind | 高精度なCPU命令数ベース測定 | あり（回帰検出） |
| **Criterion** | criterion | 実時間ベースのマイクロベンチマーク | なし（ローカル開発用） |

## Iai-Callgrind ベンチマーク

### 特徴

- **決定論的測定**: CPU命令数をカウントするため、実行環境のノイズに影響されない
- **CI向け最適化**: GitHub Actions上で安定した測定が可能
- **回帰検出**: 10%以上の性能低下を自動検出

### ベンチマークファイル

| ファイル | 対象機能 |
|----------|----------|
| `iai/persistent_vector_iai.rs` | PersistentVector操作（push, get, update, iter） |
| `iai/effect_iai.rs` | Effect System（IO, Reader, State, ExceptT, eff!マクロ） |
| `iai/scenario_iai.rs` | 複合シナリオ（モナド変換、データパイプライン、Optics等） |

### 実行方法

**前提条件**: Valgrindが必要（macOSではDocker経由で実行）

```bash
# Linux環境
cargo bench --bench persistent_vector_iai
cargo bench --bench effect_iai
cargo bench --bench scenario_iai

# 全Iai-Callgrindベンチマーク
cargo bench --bench persistent_vector_iai --bench effect_iai --bench scenario_iai

# ベースライン保存
cargo bench --bench persistent_vector_iai -- --save-baseline=main

# ベースラインとの比較
cargo bench --bench persistent_vector_iai -- --baseline=main
```

### Docker環境での実行（macOS）

macOSではValgrindが動作しないため、Docker経由で実行します:

```bash
cd docker/benchmark
docker compose run --rm benchmark cargo bench --bench persistent_vector_iai
```

### 出力の読み方

```
persistent_vector_iai::persistent_vector_group::push_back_1000
  Instructions:                 1,234,567|+5.23%     (*********)
  L1 Hits:                        987,654|+4.12%     (*********)
  L2 Hits:                         12,345|-2.34%     (      ***)
  RAM Hits:                         1,234|+0.00%     (        *)
  Total read+write:             1,001,233|+4.56%     (*********)
  Estimated Cycles:             1,345,678|+5.01%     (*********)
```

- **Instructions**: 実行されたCPU命令数（最重要指標）
- **L1/L2/RAM Hits**: キャッシュヒット数
- **Estimated Cycles**: 推定CPU サイクル数
- **パーセンテージ**: ベースラインからの変化率

## Criterion ベンチマーク

### 特徴

- **実時間測定**: 実際の実行時間を測定
- **統計分析**: 複数回実行による統計的な信頼区間を算出
- **HTMLレポート**: 詳細なグラフ付きレポートを生成

### ベンチマークファイル

| ファイル | 対象機能 |
|----------|----------|
| `persistent_vector_bench.rs` | PersistentVector |
| `persistent_hashmap_bench.rs` | PersistentHashMap |
| `persistent_treemap_bench.rs` | PersistentTreeMap |
| `persistent_list_bench.rs` | PersistentList |
| `control_bench.rs` | Lazy, Trampoline等 |
| `effect_bench.rs` | IO, Reader, State等 |
| `algebraic_effect_bench.rs` | Freer, define_effect! |
| `for_macro_bench.rs` | for_!マクロ |
| `for_async_macro_bench.rs` | for_async!マクロ |
| `bifunctor_bench.rs` | Bifunctor |
| `alternative_bench.rs` | Alternative |
| `freer_bench.rs` | Freerモナド |
| `serde_bench.rs` | シリアライズ/デシリアライズ |
| `transient_bench.rs` | Transient操作 |
| `rayon_bench.rs` | 並列処理 |

### 実行方法

```bash
# 全ベンチマーク
cargo bench

# 特定のベンチマーク
cargo bench --bench persistent_vector_bench

# 特定の関数のみ
cargo bench --bench persistent_vector_bench -- push_back

# HTMLレポート生成（自動的にtarget/criterion/に生成される）
cargo bench
open target/criterion/report/index.html  # レポートを開く

# プロット生成を無効化（高速、レポートはテキストのみ）
cargo bench -- --noplot
```

### 出力の読み方

```
push_back/1000          time:   [12.345 µs 12.456 µs 12.567 µs]
                        change: [-2.34% +0.12% +2.56%] (p = 0.05 > 0.05)
                        No change in performance detected.
```

- **time**: [下限 中央値 上限] の95%信頼区間
- **change**: 前回実行からの変化率（信頼区間）
- **p値**: 統計的有意性

## CI/CD統合

### GitHub Actionsワークフロー

| ワークフロー | トリガー | 目的 |
|--------------|----------|------|
| `benchmark.yml` | mainブランチpush | 継続的ベンチマーク記録 |
| `benchmark-pr.yml` | PR | 回帰検出（10%閾値） |

### 回帰検出

PRでは以下の場合に警告が表示されます:

- CPU命令数が10%以上増加
- ベースライン（mainブランチ）と比較

### Bencher連携

ベンチマーク結果は[Bencher](https://bencher.dev)で継続的にトラッキングされます:

- 時系列でのパフォーマンス推移
- 回帰の自動検出
- PRコメントでの結果表示

## ディレクトリ構成

```
benches/
├── README.md                      # このファイル
├── iai/                           # Iai-Callgrindベンチマーク
│   ├── persistent_vector_iai.rs
│   ├── effect_iai.rs
│   └── scenario_iai.rs
├── api/                           # タスク管理APIベンチマーク
│   └── README.md                  # API詳細
├── *_bench.rs                     # Criterionベンチマーク
└── ...
```

## 開発ガイドライン

### 新しいベンチマークを追加する場合

1. **Iai-Callgrind優先**: CI回帰検出が必要な場合はIai-Callgrindを使用
2. **Criterion**: ローカル開発時の詳細分析用に使用
3. **両方不要**: 既存のベンチマークでカバーされている場合は追加しない

### ベンチマーク関数の書き方

```rust
// Iai-Callgrind
#[library_benchmark]
fn bench_function() -> SomeType {
    let input = black_box(setup_input());
    black_box(operation(input))
}

// Criterion
fn bench_function(c: &mut Criterion) {
    c.bench_function("operation", |b| {
        b.iter(|| {
            let input = black_box(setup_input());
            black_box(operation(input))
        })
    });
}
```

### black_box の重要性

`black_box`は最適化によるベンチマークの歪みを防ぎます:

- 入力値に使用: コンパイラによる定数畳み込みを防止
- 出力値に使用: 未使用コードの削除を防止

## 参考資料

- [Iai-Callgrind Guide](https://iai-callgrind.github.io/iai-callgrind/latest/html/index.html)
- [Criterion User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Bencher Documentation](https://bencher.dev/docs/)
