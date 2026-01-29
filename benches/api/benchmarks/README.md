# API Benchmark Suite

lambars の API ベンチマークスイートです。wrk2 を使用して HTTP ベンチマークを実行します。

## 前提条件

### wrk2 のインストール

このベンチマークスイートは [wrk2](https://github.com/giltene/wrk2) を使用します。
wrk2 は wrk のフォークで、正確なレート制御 (`-R` オプション) をサポートしています。

**重要**: 通常の `wrk` ではなく `wrk2` が必要です。

#### 自動インストール

セットアップスクリプトを使用してインストールできます:

```bash
./setup_wrk2.sh
```

オプション:
- `--force`: 既にインストール済みでも再インストール

#### 手動インストール

##### macOS (Homebrew)

```bash
brew tap cfdrake/tap
brew install cfdrake/tap/wrk2
```

または、ソースからビルド:

```bash
# OpenSSL が必要
brew install openssl

# ビルド
git clone --depth 1 https://github.com/giltene/wrk2.git
cd wrk2
export LDFLAGS="-L$(brew --prefix openssl)/lib"
export CPPFLAGS="-I$(brew --prefix openssl)/include"
make -j$(sysctl -n hw.ncpu)
sudo cp wrk /usr/local/bin/wrk2
```

##### Linux (Ubuntu/Debian)

```bash
# ビルド依存関係のインストール
sudo apt-get update
sudo apt-get install -y build-essential libssl-dev git

# ビルド
git clone --depth 1 https://github.com/giltene/wrk2.git
cd wrk2
make -j$(nproc)
sudo cp wrk /usr/local/bin/wrk2
```

##### インストール確認

```bash
wrk2 -v
# 出力例: wrk 4.2.0 [epoll] Copyright (C) 2012 Will Glozer
```

### Docker

ベンチマーク対象の API サーバーを起動するために Docker が必要です。

```bash
# API サーバーを起動
cd ../docker
docker compose -f compose.ci.yaml up -d --build --wait
```

## ディレクトリ構成

```
benchmarks/
├── scripts/           # Lua スクリプト (25 ファイル)
├── scenarios/         # ベンチマークシナリオ定義 (YAML)
├── results/           # ベンチマーク結果
├── schema/            # JSON スキーマ
├── templates/         # レポートテンプレート
├── run_benchmark.sh   # メインベンチマークスクリプト
├── setup_wrk2.sh      # wrk2 インストールスクリプト
├── setup_test_data.sh # テストデータセットアップ
├── test_lua_compatibility.sh  # Lua 互換性テスト
├── check_thresholds.sh        # しきい値チェック
├── compare_results.sh         # 結果比較
├── validate_meta_schema.sh    # メタスキーマ検証
├── thresholds.yaml    # しきい値定義
└── nightly_config.yaml # Nightly 設定
```

## 使い方

### 基本的な使い方

```bash
# テストデータをセットアップ
./setup_test_data.sh --scale small

# ベンチマークを実行
./run_benchmark.sh --scenario scenarios/tasks_eff.yaml
```

### Lua スクリプト互換性テスト

全ての Lua スクリプトが wrk2 で正常に動作することを確認します:

```bash
# API サーバーが起動している状態で実行
./test_lua_compatibility.sh

# オプション
./test_lua_compatibility.sh --target http://localhost:8080 --duration 5 --verbose
```

オプション:
- `--target URL`: テスト対象の URL (デフォルト: `http://localhost:8080`)
- `--duration SECONDS`: 各スクリプトの実行時間 (デフォルト: 5)
- `--rate RPS`: リクエストレート (デフォルト: 10)
- `--skip-server`: サーバー起動チェックをスキップ
- `--verbose, -v`: 詳細出力

### 結果の比較

```bash
./compare_results.sh results/run1/meta.json results/run2/meta.json
```

### しきい値チェック

```bash
./check_thresholds.sh results/latest/meta.json
```

## Lua スクリプト

各 Lua スクリプトは特定のベンチマークシナリオを実装しています:

| スクリプト | 説明 |
|-----------|------|
| `alternative.lua` | Alternative 型クラスベンチマーク |
| `applicative.lua` | Applicative 型クラスベンチマーク |
| `async_pipeline.lua` | 非同期パイプラインベンチマーク |
| `bifunctor.lua` | Bifunctor 型クラスベンチマーク |
| `cache_metrics.lua` | キャッシュメトリクスベンチマーク |
| `common.lua` | 共通ユーティリティ |
| `contention.lua` | 競合状態ベンチマーク |
| `error_tracker.lua` | エラートラッキング |
| `load_profile.lua` | 負荷プロファイルベンチマーク |
| `load_shape_demo.lua` | 負荷形状デモ |
| `misc.lua` | その他のベンチマーク |
| `optics.lua` | Optics ベンチマーク |
| `ordered.lua` | 順序付きベンチマーク |
| `payload_generator.lua` | ペイロード生成 |
| `profile_wrk.lua` | wrk プロファイリング |
| `projects_progress.lua` | プロジェクト進捗ベンチマーク |
| `recursive.lua` | 再帰処理ベンチマーク |
| `result_collector.lua` | 結果収集 |
| `seed_data.lua` | シードデータ |
| `tasks_bulk.lua` | タスク一括処理ベンチマーク |
| `tasks_eff.lua` | タスク Effect ベンチマーク |
| `tasks_search.lua` | タスク検索ベンチマーク |
| `tasks_update.lua` | タスク更新ベンチマーク |
| `test_ids.lua` | テスト ID |
| `traversable.lua` | Traversable 型クラスベンチマーク |

## wrk2 と wrk の違い

wrk2 は wrk のフォークで、以下の機能が追加されています:

1. **正確なレート制御** (`-R` オプション): 指定したリクエストレートを維持
2. **Coordinated Omission 対策**: より正確なレイテンシ測定
3. **HdrHistogram サポート**: 高精度なパーセンタイル測定

```bash
# wrk の例 (レート制御なし)
wrk -t4 -c100 -d30s http://localhost:8080/api/tasks

# wrk2 の例 (1000 req/sec でレート制御)
wrk2 -t4 -c100 -d30s -R1000 http://localhost:8080/api/tasks
```

## CI/CD

GitHub Actions で自動ベンチマークが実行されます:

- `benchmark-baseline`: main ブランチのベースラインベンチマーク
- `benchmark-pr`: PR のベンチマーク
- `benchmark-main`: main ブランチへのマージ後ベンチマーク
- `benchmark-manual`: 手動トリガーベンチマーク
- `benchmark-nightly`: 毎日のフルカバレッジベンチマーク

## プロファイリング

### perf によるプロファイリング（Linux）

ベンチマークと同時に `perf` によるプロファイリングが可能です。

```bash
# プロファイリング付きベンチマーク実行
PROFILE_MODE=true ./run_benchmark.sh --scenario scenarios/tasks_eff.yaml

# または scenarios YAML で設定
# profiling:
#   enable_perf: true
#   enable_flamegraph: true
```

#### シンボル解決の設定

`perf` と flamegraph でシンボルを正しく解決するため、以下の設定を行っています:

1. **デバッグシンボルの埋め込み**
   - `benches/api/Cargo.toml` および ルート `Cargo.toml` の `[profile.release]` に `debug = 1` を設定
   - `debug = 1`: Line tables のみ（関数名・行番号）
   - バイナリサイズへの影響: 約 5-10% 増加
   - より詳細なプロファイリングが必要な場合は `debug = 2` に変更（バイナリサイズ 30-50% 増加）

2. **call-graph の取得方法**
   - デフォルト: `--call-graph dwarf,16384` (DWARF デバッグ情報、16KB スタック)
   - フォールバック: `-g` (frame pointer)
   - 自動判定により環境に応じた最適な方法を選択

3. **build-id によるシンボル解決**
   - Rust のデフォルトで build-id が埋め込まれます
   - strip されていないバイナリを使用することで、perf がシンボルを解決可能

4. **macOS での profiling**
   - macOS では `perf` の代わりに `sample` コマンドを使用
   - `debug = 1` により最低限のシンボル情報が埋め込まれます
   - より詳細な情報が必要な場合は `Instruments.app` の使用を推奨

#### 制限事項と注意点

- **DWARF サポート**: 古い perf バージョンでは `--call-graph dwarf` が非対応の場合があります（自動フォールバック）
- **CPU オーバーヘッド**: DWARF 取得は frame pointer より CPU 負荷が高いため、ベンチマーク結果に影響する可能性があります
- **[unknown] シンボル**: 以下の条件で発生する可能性があります
  - バイナリが strip されている
  - インライン展開された関数（`debug = 1` では解決不可）
  - JIT コンパイルされたコード

## トラブルシューティング

### wrk2 が見つからない

```bash
# インストール確認
which wrk2
wrk2 -v

# 再インストール
./setup_wrk2.sh --force
```

### Lua スクリプトエラー

```bash
# 互換性テストを実行
./test_lua_compatibility.sh --verbose

# 特定のスクリプトをテスト
wrk2 -t1 -c1 -d5s -R10 -s scripts/tasks_eff.lua http://localhost:8080
```

### サーバー接続エラー

```bash
# サーバーが起動しているか確認
curl http://localhost:8080/health

# Docker コンテナの状態を確認
cd ../docker
docker compose -f compose.ci.yaml ps
docker compose -f compose.ci.yaml logs
```
