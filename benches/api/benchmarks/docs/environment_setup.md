# ベンチマーク環境ノイズ対策ガイド

## 概要

ベンチマーク測定の信頼性を確保するため、環境ノイズを最小化する方法をまとめたガイドです。環境ノイズとは、測定対象以外の要因（バックグラウンドプロセス、CPU周波数変動、ディスクI/Oなど）による測定値の変動を指します。

## 環境ノイズの影響

環境ノイズが大きいと、以下の問題が発生します。

### 測定値のばらつき増加

- 標準偏差が大きくなる
- 95%信頼区間の幅が広がる
- 収束条件（CI幅 < 10%）を満たすまでの反復回数が増加

### 誤検知の増加

- 真の性能退行ではないのに、退行として検出される
- 真の性能改善が、ノイズに埋もれて検出されない

### 測定時間の増加

- 収束しない場合、最大10回の反復が必要
- CI/CD パイプラインの実行時間が増加

## 環境ノイズの要因と対策

### 1. CPU 周波数の変動

#### 要因

- **Turbo Boost / Precision Boost**: 負荷に応じてCPU周波数が動的に変化
- **省電力モード**: バッテリー駆動時に周波数が低下
- **温度調整**: CPU温度が上昇すると周波数が低下（サーマルスロットリング）

#### 影響

CPU周波数が変動すると、同じコードでも実行時間が変わり、測定値がばらつきます。

#### 対策（Linux）

**CPU Governor を performance に設定**:

```bash
# 現在の Governor を確認
cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# performance モードに設定（全コア）
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Turbo Boost を無効化（Intel CPU）
echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo

# Turbo Boost を無効化（AMD CPU）
echo 0 | sudo tee /sys/devices/system/cpu/cpufreq/boost
```

**永続化**（再起動後も有効）:

```bash
# /etc/rc.local または systemd service に設定を追加
sudo tee /etc/rc.local <<EOF
#!/bin/bash
echo performance | tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
echo 1 | tee /sys/devices/system/cpu/intel_pstate/no_turbo
exit 0
EOF

sudo chmod +x /etc/rc.local
```

#### 対策（macOS）

macOS では CPU Governor の制御が制限されています。

**推奨事項**:
- AC電源に接続（バッテリー駆動を避ける）
- システム環境設定 → バッテリー → 省エネルギー → 「プロセッサのパフォーマンス」を「高」に設定
- 統計的手法（反復測定）でノイズを吸収

---

### 2. バックグラウンドプロセス

#### 要因

- **ブラウザ**: Chrome, Firefox などがCPUとメモリを消費
- **IDEやエディタ**: VSCode, IntelliJ などのインデックス作成
- **システムサービス**: cron, Spotlight（macOS）, Windows Update など
- **コンテナやVM**: Docker, VirtualBox などのオーバーヘッド

#### 影響

バックグラウンドプロセスがCPUやメモリを使用すると、ベンチマーク対象のプロセスに割り当てられるリソースが減少し、測定値が変動します。

#### 対策

**不要なプロセスを停止**:

```bash
# Linux: CPU使用率が高いプロセスを確認
top -o %CPU

# macOS: CPU使用率が高いプロセスを確認
top -o cpu

# プロセスを停止
sudo systemctl stop <service-name>  # Linux (systemd)
killall <process-name>              # 任意のOS
```

**推奨停止対象**:
- ブラウザ（Chrome, Firefox）
- IDEやエディタ（VSCode, IntelliJ）
- チャットアプリ（Slack, Discord）
- 音楽プレーヤー（Spotify）
- クラウド同期（Dropbox, Google Drive）

**Docker コンテナの最適化**:

```bash
# 不要なコンテナを停止
docker ps -a
docker stop <container-id>

# Docker のリソース制限を設定
docker run --cpus="2.0" --memory="4g" <image>
```

---

### 3. CPU ピニング（コア固定）

#### 要因

- **スケジューラのコンテキストスイッチ**: プロセスが異なるCPUコア間で移動すると、キャッシュの無効化が発生
- **NUMA ノード**: 異なるNUMAノード間でメモリアクセスが発生すると、レイテンシが増加

#### 影響

プロセスが複数のCPUコア間で移動すると、L1/L2/L3キャッシュの内容が無効化され、測定値が変動します。

#### 対策（Linux）

**taskset でCPUコアを固定**:

```bash
# CPU 0-3 にプロセスを固定
taskset -c 0-3 wrk2 -t2 -c10 -d30s http://localhost:3002

# または
taskset -c 0-3 ./run_benchmark.sh --scenario scenarios/tasks_eff.yaml
```

**numactl でNUMAノードを固定**:

```bash
# NUMA ノード 0 に固定
numactl --cpunodebind=0 --membind=0 wrk2 -t2 -c10 -d30s http://localhost:3002

# NUMA 構成を確認
numactl --hardware
```

#### 対策（macOS）

macOS では `taskset` や `numactl` が使用できません。

**代替手段**:
- 統計的手法（反復測定）でノイズを吸収
- `nice` コマンドで優先度を上げる:

```bash
sudo nice -n -20 wrk2 -t2 -c10 -d30s http://localhost:3002
```

---

### 4. ディスク I/O の影響

#### 要因

- **ログ出力**: 大量のログがディスクに書き込まれる
- **スワップ**: メモリ不足でスワップが発生
- **バックグラウンド処理**: Spotlight（macOS）、Windows Search など

#### 影響

ディスクI/Oが発生すると、CPUがI/O待ちになり、測定値が変動します。

#### 対策

**ログレベルを下げる**:

```bash
# API サーバーのログレベルを ERROR に設定
RUST_LOG=error ./target/release/api-server
```

**スワップを無効化**（一時的）:

```bash
# Linux
sudo swapoff -a

# 測定後に再有効化
sudo swapon -a
```

**メモリ使用量を監視**:

```bash
# Linux
free -m

# macOS
vm_stat
```

**十分なメモリを確保**:
- 推奨: メモリ使用量 < 利用可能メモリの 80%

---

### 5. ネットワークの影響

#### 要因

- **ネットワーク遅延**: WiFi、VPN などの不安定なネットワーク
- **ファイアウォール**: パケットフィルタリングのオーバーヘッド
- **他のネットワークトラフィック**: ダウンロード、ストリーミングなど

#### 影響

ネットワーク遅延が変動すると、API ベンチマーク（wrk2）の測定値がばらつきます。

#### 対策

**ローカルホストを使用**:

```bash
# ローカルホスト経由で測定（推奨）
wrk2 -t2 -c10 -d30s http://localhost:3002

# 127.0.0.1 を明示的に指定
wrk2 -t2 -c10 -d30s http://127.0.0.1:3002
```

**ネットワークを無効化**（可能な場合）:

```bash
# WiFi を無効化（macOS）
networksetup -setairportpower en0 off

# 測定後に再有効化
networksetup -setairportpower en0 on
```

**他のネットワークトラフィックを停止**:
- ブラウザのダウンロードを停止
- ストリーミング（YouTube, Netflix）を停止
- クラウド同期（Dropbox, Google Drive）を停止

---

### 6. JIT ウォームアップの不足

#### 要因

- **コールドスタート**: 最初のリクエストでJITコンパイルやキャッシュのロードが発生
- **キャッシュミス**: 初回アクセス時にキャッシュがまだ構築されていない

#### 影響

ウォームアップが不足すると、最初の数回の測定値が異常に遅く、統計的信頼性が低下します。

#### 対策

**ウォームアップ期間を設ける**:

```bash
# 注: run_benchmark_with_stats.sh は今後実装予定
# 現時点では wrk2 を直接実行し、最初の数秒を無視してください

# 10秒のウォームアップ（将来の実装）
# ./run_benchmark_with_stats.sh --warmup-duration 10 --scenario scenarios/tasks_eff.yaml
```

**Criterion のウォームアップ**:

Criterion はデフォルトで3秒のウォームアップを実行します。必要に応じて追加のウォームアップを設定します。

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn my_benchmark(c: &mut Criterion) {
    c.bench_function("my_function", |b| {
        // 追加のウォームアップ
        for _ in 0..100 {
            my_function();
        }

        // 測定
        b.iter(|| my_function());
    });
}

criterion_group!(benches, my_benchmark);
criterion_main!(benches);
```

---

## 推奨環境設定

### 最小構成（必須）

以下の設定は、どの環境でも実施すべき最小限の対策です。

1. **バックグラウンドプロセスの停止**
   - ブラウザ、IDE、チャットアプリを停止

2. **ローカルホストの使用**
   - ネットワーク経由ではなく、ローカルホストで測定

3. **ウォームアップ期間の設定**
   - 10秒のウォームアップを実行

4. **メモリの確保**
   - メモリ使用量 < 利用可能メモリの 80%

### 推奨構成（高精度測定用）

より高精度な測定が必要な場合、以下の追加対策を実施します。

1. **CPU Governor を performance に設定**（Linux）
   - Turbo Boost を無効化

2. **CPU ピニング**（Linux）
   - `taskset` または `numactl` でCPUコアを固定

3. **ログレベルを下げる**
   - API サーバーのログを ERROR レベルに設定

4. **反復測定の増加**
   - 最小3回、最大10回の反復測定

### 理想構成（CI/CD 専用環境）

CI/CD パイプラインで安定した測定を行うための理想的な環境設定です。

1. **Self-hosted runner の使用**
   - GitHub Actions の self-hosted runner を専用マシンで実行
   - 他のジョブと同時実行しない（queue 制御）

2. **ベアメタル環境**
   - Docker や VM のオーバーヘッドを避ける
   - または、十分なリソース（CPU, メモリ）を割り当てる

3. **環境の固定化**
   - CPU モデル、メモリ、ディスクを固定
   - OS バージョン、カーネルバージョンを固定

4. **自動化スクリプト**
   - 環境設定（CPU Governor, プロセス停止）を自動化
   - 測定前チェックスクリプトの実行

---

## CI/CD 環境での設定方法

### GitHub Actions（Ubuntu）

```yaml
name: Benchmark

on:
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest  # または self-hosted

    steps:
      - uses: actions/checkout@v3

      - name: Setup environment
        run: |
          # CPU Governor を performance に設定
          echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

          # Turbo Boost を無効化
          echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo

          # 不要なサービスを停止
          sudo systemctl stop unattended-upgrades
          sudo systemctl stop snapd

      - name: Check system resources
        run: |
          # CPU 情報
          lscpu

          # メモリ情報
          free -m

          # ディスク情報
          df -h

      - name: Run benchmark
        run: |
          cd benches/api/benchmarks
          ./scripts/run_benchmark_with_stats.sh \
            --scenario scenarios/tasks_eff.yaml \
            --output results/${{ github.sha }}

      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: benches/api/benchmarks/results/${{ github.sha }}
```

### Self-hosted Runner の推奨設定

**ハードウェア**:
- CPU: 4コア以上（固定クロック）
- メモリ: 16GB以上
- ディスク: SSD

**ソフトウェア**:
- OS: Ubuntu 22.04 LTS（固定バージョン）
- カーネル: 固定バージョン
- Docker: 最新安定版

**環境設定**:
- CPU Governor: performance
- Turbo Boost: 無効
- Swap: 無効
- ファイアウォール: 無効（ローカルホストのみ使用）

---

## 測定前チェックリスト

ベンチマーク測定を実行する前に、以下のチェックリストで環境を確認します。

### 必須項目

- [ ] バックグラウンドプロセスを停止した（ブラウザ、IDE、チャット）
- [ ] ローカルホストで測定する（ネットワーク経由ではない）
- [ ] ウォームアップ期間を設定した（10秒以上）
- [ ] メモリ使用量が上限以下（< 利用可能メモリの 80%）

### 推奨項目（Linux）

- [ ] CPU Governor を performance に設定した
- [ ] Turbo Boost を無効化した
- [ ] CPU ピニングを設定した（`taskset` または `numactl`）
- [ ] ログレベルを ERROR に設定した

### 理想項目（CI/CD）

- [ ] Self-hosted runner を使用している
- [ ] ベアメタル環境で実行している
- [ ] 環境が固定化されている（CPU、メモリ、OS）
- [ ] 自動化スクリプトで環境設定を行っている

---

## 環境確認スクリプト

測定前に環境を自動チェックするスクリプトの例です。

```bash
#!/bin/bash
# check_environment.sh

set -e

echo "=== ベンチマーク環境チェック ==="

# CPU Governor
echo ""
echo "[CPU Governor]"
governors=$(cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor 2>/dev/null || echo "N/A")
if echo "$governors" | grep -q "performance"; then
    echo "✓ performance モードに設定されています"
else
    echo "⚠ performance モードではありません: $governors"
    echo "  推奨: echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor"
fi

# Turbo Boost
echo ""
echo "[Turbo Boost]"
turbo=$(cat /sys/devices/system/cpu/intel_pstate/no_turbo 2>/dev/null || echo "N/A")
if [ "$turbo" == "1" ]; then
    echo "✓ Turbo Boost が無効化されています"
elif [ "$turbo" == "N/A" ]; then
    echo "⚠ Turbo Boost の状態を確認できません（AMD CPU または macOS）"
else
    echo "⚠ Turbo Boost が有効です"
    echo "  推奨: echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo"
fi

# メモリ
echo ""
echo "[メモリ]"
if command -v free &> /dev/null; then
    free -m
    available=$(free -m | awk 'NR==2 {print $7}')
    total=$(free -m | awk 'NR==2 {print $2}')
    usage=$((100 - available * 100 / total))
    if [ $usage -lt 80 ]; then
        echo "✓ メモリ使用率: ${usage}%（利用可能メモリの 80% 未満）"
    else
        echo "⚠ メモリ使用率: ${usage}%（利用可能メモリの 80% 以上）"
        echo "  推奨: 不要なプロセスを停止してください"
    fi
fi

# バックグラウンドプロセス
echo ""
echo "[高CPU使用プロセス（上位5件）]"
if command -v ps &> /dev/null; then
    ps aux --sort=-%cpu | head -6 | tail -5
    echo "  推奨: ブラウザ、IDE、チャットアプリを停止してください"
fi

echo ""
echo "=== チェック完了 ==="
```

**使用方法**:

```bash
# 注: 上記のスクリプトは今後実装予定
# 現時点ではスクリプトを作成して使用してください

chmod +x check_environment.sh
./check_environment.sh
```

---

## トラブルシューティング

### 測定値が収束しない（CI幅 > 10%）

**原因**:
- 環境ノイズが大きい
- ウォームアップ不足
- 測定時間が短すぎる

**対策**:
1. 環境ノイズ対策を実施（CPU Governor、プロセス停止など）
2. ウォームアップ時間を延長（20秒に設定）
3. 測定時間を延長（`-d 60s` に設定）
4. 反復回数の上限を増やす（最大10回、要件で規定）

### CPU Governor が変更できない

**原因**:
- 権限不足（`sudo` が必要）
- CPU が対応していない（一部のVM環境）

**対策**:
1. `sudo` で実行
2. VM環境の場合、ホストOSで設定
3. または、統計的手法（反復測定）でノイズを吸収

### macOS で CPU 制御ができない

**原因**:
- macOS は CPU Governor の制御が制限されている

**対策**:
1. AC電源に接続
2. 省エネルギー設定を「高パフォーマンス」に変更
3. 統計的手法（反復測定）でノイズを吸収
4. 可能であれば Linux 環境で測定

---

## 参考資料

- [ベンチマーク網羅性改善 要件定義](../../../docs/internal/requirements/20260201_1300_benchmark_coverage_improvement.yaml)
- [統計結果フォーマット定義](./stats_format.md)
- [Linux CPU Governor Documentation](https://www.kernel.org/doc/Documentation/cpu-freq/governors.txt)
- [Intel Turbo Boost Technology](https://www.intel.com/content/www/us/en/architecture-and-technology/turbo-boost/turbo-boost-technology.html)
- [NUMA (Non-Uniform Memory Access)](https://en.wikipedia.org/wiki/Non-uniform_memory_access)
