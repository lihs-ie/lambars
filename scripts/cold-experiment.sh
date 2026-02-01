#!/usr/bin/env bash
set -euo pipefail

get_cpu_info() {
    if command -v lscpu >/dev/null 2>&1; then
        lscpu | grep "Model name" | sed 's/Model name:[[:space:]]*//'
    elif command -v sysctl >/dev/null 2>&1; then
        sysctl -n machdep.cpu.brand_string 2>/dev/null || echo "Unknown CPU"
    else
        uname -m
    fi
}

show_help() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

コールドスタートの影響を検証するスクリプト。
キャッシュをクリアして複数回実行し、結果の安定性を確認します。

OPTIONS:
    --bench NAME         ベンチマーク名 [default: persistent_vector]
    --runs N             実行回数 [default: 10]
    --output DIR         出力ディレクトリ [default: benches/results/cold-experiment]
    --help               ヘルプを表示

EXAMPLES:
    scripts/cold-experiment.sh
    scripts/cold-experiment.sh --runs 20 --output /tmp/results
EOF
}

BENCH_NAME="persistent_vector"
RUNS=10
OUTPUT_DIR="benches/results/cold-experiment"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --bench)
            if [[ $# -lt 2 ]]; then
                echo "Error: --bench requires an argument" >&2
                show_help
                exit 1
            fi
            BENCH_NAME="$2"; shift 2 ;;
        --runs)
            if [[ $# -lt 2 ]]; then
                echo "Error: --runs requires an argument" >&2
                show_help
                exit 1
            fi
            RUNS="$2"; shift 2 ;;
        --output)
            if [[ $# -lt 2 ]]; then
                echo "Error: --output requires an argument" >&2
                show_help
                exit 1
            fi
            OUTPUT_DIR="$2"; shift 2 ;;
        --help) show_help; exit 0 ;;
        *) echo "Error: Unknown option: $1" >&2; show_help; exit 1 ;;
    esac
done

if ! [[ "$RUNS" =~ ^[0-9]+$ ]] || [ "$RUNS" -lt 10 ]; then
    echo "Error: Invalid runs: $RUNS (must be >= 10 for REQ-PV-PUSH-005)" >&2
    exit 1
fi

mkdir -p "$OUTPUT_DIR"

if [[ -f "scripts/benchmark-env.sh" ]]; then
    # shellcheck source=scripts/benchmark-env.sh
    source scripts/benchmark-env.sh
else
    echo "Warning: benchmark-env.sh not found, using defaults" >&2
    export RUSTFLAGS="${RUSTFLAGS:--C lto=fat -C codegen-units=1 -C opt-level=3}"
    export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/benchmark}"
fi

echo "=== Cold Start Experiment ==="
echo "Benchmark: $BENCH_NAME / Runs: $RUNS / Output: $OUTPUT_DIR"
echo ""

METADATA_FILE="$OUTPUT_DIR/metadata.txt"
{
    echo "=== Experiment Metadata ==="
    echo "Date: $(date -u +"%Y-%m-%d %H:%M:%S UTC")"
    echo "Rustc Version: $(rustc --version)"
    echo "CPU: $(get_cpu_info)"
    echo "OS: $(uname -s) $(uname -r)"
    echo "RUSTFLAGS: ${RUSTFLAGS:-<not set>}"
    echo "CARGO_TARGET_DIR: ${CARGO_TARGET_DIR:-<not set>}"
    echo "Benchmark: $BENCH_NAME"
    echo "Runs: $RUNS"
    echo ""
    echo "=== Cargo.lock Hash ==="
    if [[ -f "Cargo.lock" ]]; then
        sha256sum Cargo.lock 2>/dev/null || shasum -a 256 Cargo.lock 2>/dev/null || echo "sha256sum not available"
    else
        echo "Cargo.lock not found"
    fi
    echo ""
    echo "=== Cargo Profile (release) ==="
    if [[ -f "Cargo.toml" ]]; then
        grep -A 10 "\[profile.release\]" Cargo.toml || echo "No [profile.release] section"
    else
        echo "Cargo.toml not found"
    fi
} > "$METADATA_FILE"
echo "Metadata saved to: $METADATA_FILE"
echo ""

RESULTS_FILE="$OUTPUT_DIR/cold-runs.csv"
echo "run,benchmark,instructions" > "$RESULTS_FILE"

for i in $(seq 1 "$RUNS"); do
    echo "=== Run $i/$RUNS ==="
    LOG_FILE="$OUTPUT_DIR/run-${i}.log"

    cargo clean
    if cargo bench --bench "$BENCH_NAME" -- --save-baseline "cold-run-$i" 2>&1 | tee "$LOG_FILE"; then
        echo "✓ Run $i"
        if grep -q "push_back_1000" "$LOG_FILE"; then
            # Extract instructions from the line after "push_back_1000" (IAI Callgrind format)
            # Use || true to continue on format mismatch
            INSTRUCTIONS=$(grep -A 1 "push_back_1000" "$LOG_FILE" | grep "Instructions:" | grep -o '[0-9,]\+' | tr -d ',' | head -1 || true)
            [[ -n "$INSTRUCTIONS" ]] && echo "$i,push_back_1000,$INSTRUCTIONS" >> "$RESULTS_FILE"
        fi
    else
        echo "✗ Run $i" >&2
    fi
    echo ""
done

if [[ -f "scripts/benchmark-stats.sh" ]]; then
    echo "=== Generating Statistics ==="

    # Check if we have enough data points (REQ-PV-PUSH-005: 10 runs)
    DATA_COUNT=$(tail -n +2 "$RESULTS_FILE" | wc -l | tr -d ' ')
    if [[ "$DATA_COUNT" -lt "$RUNS" ]]; then
        echo "✗ Insufficient data: $DATA_COUNT/$RUNS runs succeeded" >&2
        echo "=== Complete with errors ===" >&2
        echo "Results: $OUTPUT_DIR" >&2
        exit 1
    fi

    bash scripts/benchmark-stats.sh --input "$RESULTS_FILE" --output "$OUTPUT_DIR/statistics.txt"

    # Check CV threshold (REQ-PV-PUSH-005: CV <= 3%)
    if grep -q "CV:" "$OUTPUT_DIR/statistics.txt"; then
        CV_VALUE=$(grep "CV:" "$OUTPUT_DIR/statistics.txt" | grep -o '[0-9.]\+' | head -1)
        if awk -v cv="$CV_VALUE" 'BEGIN { exit (cv > 3.0) }'; then
            echo "✓ CV check passed: $CV_VALUE% <= 3.0%"
        else
            echo "✗ CV check failed: $CV_VALUE% > 3.0%" >&2
            echo "=== Complete with errors ===" >&2
            echo "Results: $OUTPUT_DIR" >&2
            exit 1
        fi
    fi
fi

echo "=== Complete ==="
echo "Results: $OUTPUT_DIR"
