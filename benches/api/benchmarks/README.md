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

### シナリオ設定

シナリオ YAML ファイルには複数のレイヤーで設定を記述します。

#### `metadata.api_features` (ドキュメント用)
```yaml
metadata:
  api_features: [for_!, Alternative]
```
ベンチマークが測定対象とする lambars API の機能を記述（レポート生成、結果分類用）。

#### `docker_build.api_features` (ビルド時)
```yaml
docker_build:
  api_features: "mimalloc,fast-hash"
```
Docker ビルド時の Cargo features（allocator, hasher など）。CI nightly でシナリオごとに適用。

#### `docker_runtime.writer_profile` (実行時)
```yaml
docker_runtime:
  writer_profile: "bulk"
```
コンテナ実行時の環境変数（API サーバーのバッチサイズ、タイムアウトなど）。

#### `thresholds.min_rps_achieved` (CI RPS ゲート)
```yaml
thresholds:
  min_rps_achieved: 400
```
CI nightly の tasks_bulk シナリオで fail-closed RPS ゲートチェックに使用される最小 RPS 閾値。
この値を下回った場合、CI は失敗します（`.github/workflows/benchmark-api.yml` で評価）。

**注意**: `docker_build` / `docker_runtime` / `thresholds.min_rps_achieved` は CI 専用拡張キーです。
`cargo xtask bench-api` の `ScenarioConfig` では未知キーは許容されますが、
API サーバー内部の `BenchmarkScenario` 型（`serde(deny_unknown_fields)`）を使う経路では
未知キーはエラーになります。CI では `yq` で直接読み取るため動作します。

### 環境変数

ベンチマークの動作を制御する環境変数を以下に示します。
これらの環境変数は、シナリオファイルの `metadata` セクションを優先し、
トップレベルの設定値や `error_config` セクション、デフォルト値で補完されます。
手動で上書きすることも可能です。

#### tasks_update 関連の環境変数

| 環境変数 | 説明 | デフォルト値 | シナリオファイルでの設定 |
|---------|------|-------------|----------------------|
| `ID_POOL_SIZE` | 更新ベンチマークで使用するタスク ID の数。小さい値ほど競合が増加する。 | 10 | `metadata.id_pool_size` または `id_pool_size` |
| `RETRY_COUNT` | 409 Conflict 発生時の最大再試行回数 | 0 | `metadata.retry_count` または `error_config.max_retries` |
| `WRK_THREADS` | wrk スレッド数（Lua スクリプトのスレッドローカル状態用）。通常は wrk の `-t` オプションと同じ値を設定する。 | `THREADS` と同じ | 自動設定 |
| `RETRY_BACKOFF_MAX` | 擬似指数バックオフの上限（スキップするリクエスト数）。再試行時のバックオフ遅延を制御する。 | 16 | 手動設定のみ |

#### 使用例

```bash
# シナリオファイルで設定（推奨）
# scenarios/tasks_update.yaml
metadata:
  id_pool_size: 1000
  retry_count: 3

# 環境変数で上書き
ID_POOL_SIZE=500 RETRY_COUNT=5 ./run_benchmark.sh --scenario scenarios/tasks_update.yaml

# バックオフ上限を調整
RETRY_BACKOFF_MAX=32 ./run_benchmark.sh --scenario scenarios/tasks_update.yaml
```

#### 重要な注意事項

**tasks_update シナリオの制約:**
- `threads == connections` が必須です（1 thread = 1 connection）
- version 状態管理がスレッド単位で行われるため、threads != connections の場合は version 不整合により 409 Conflict が多発します
- HTTP Status Distribution は単一スレッドからの推定値です（wrk2 の制約により全スレッド集計は不可）
- 詳細は `scripts/tasks_update.lua` のコメントを参照してください

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
./check_thresholds.sh results/latest tasks_bulk
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

## HTTP ステータス集計

ベンチマーク実行時に HTTP ステータスコードを集計し、エラー原因の分析を可能にします。

### 出力ファイル

| ファイル | 説明 |
|---------|------|
| `lua_metrics.json` | Lua スクリプトが生成する HTTP ステータス集計 |
| `raw_wrk.txt` | wrk の生出力（done ハンドラ出力を含む） |
| `meta.json` | 最終成果物（http_status を含む） |

### lua_metrics.json のフォーマット

```json
{
  "total_requests": 1000,
  "error_rate": 0.05,
  "http_status": {
    "200": 800,
    "201": 150,
    "400": 30,
    "409": 15,
    "500": 5
  },
  "status_distribution": {
    "200": 0.80,
    "201": 0.15,
    "400": 0.03,
    "409": 0.015,
    "500": 0.005
  },
  "latency": {
    "min_ms": 1.5,
    "max_ms": 150.0,
    "mean_ms": 10.5,
    "p50_ms": 8.2,
    "p99_ms": 45.3
  }
}
```

### パイプラインテスト

HTTP ステータス集計パイプラインの動作確認:

```bash
# 既存結果のテスト
./scripts/test_http_status_pipeline.sh

# ベンチマーク実行してからテスト (API サーバー必須)
./scripts/test_http_status_pipeline.sh --run
```

### アーキテクチャ

```
wrk (Lua)
  | track_thread_response()
error_tracker.lua (スレッド状態管理)
  | get_thread_aggregated_summary()
common.lua (done ハンドラ)
  | io.write()
raw_wrk.txt (標準出力)

result_collector.lua
  | finalize() -> save_results()
lua_metrics.json (各フェーズ)
  | merge_lua_metrics.py
lua_metrics.json (統合)
  | generate_meta_json()
meta.json (最終成果物)
```

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

### http_status が空の場合

1. lua_metrics.json が存在するか確認:
   ```bash
   find results/ -name "lua_metrics.json"
   ```

2. raw_wrk.txt に done 出力があるか確認:
   ```bash
   grep "HTTP Status Distribution" results/*/raw_wrk.txt
   ```

3. LUA_RESULTS_DIR が設定されているか確認:
   ```bash
   env | grep LUA_RESULTS_DIR
   ```

4. パイプラインテストを実行:
   ```bash
   ./scripts/test_http_status_pipeline.sh
   ```

### done ハンドラが実行されない場合

- Lua スクリプトに `done()` 関数が定義されているか確認
- `common.finalize_benchmark()` が呼ばれているか確認
- wrk の `-d` オプションで十分な実行時間が指定されているか確認
