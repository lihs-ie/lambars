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

インライン化の影響を検証するスクリプト。#[inline] 追加前後を比較します。

OPTIONS:
    --baseline NAME      ベースライン名 [default: before-inline]
    --comparison NAME    比較対象名 [default: after-inline]
    --bench NAME         ベンチマーク名 [default: persistent_vector]
    --output DIR         出力ディレクトリ [default: benches/results/inline-experiment]
    --help               ヘルプを表示

EXAMPLES:
    scripts/inline-experiment.sh
    scripts/inline-experiment.sh --baseline my-baseline --output /tmp/results
EOF
}

BASELINE_NAME="before-inline"
COMPARISON_NAME="after-inline"
BENCH_NAME="persistent_vector"
OUTPUT_DIR="benches/results/inline-experiment"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --baseline)
            if [[ $# -lt 2 ]]; then
                echo "Error: --baseline requires an argument" >&2
                show_help
                exit 1
            fi
            BASELINE_NAME="$2"; shift 2 ;;
        --comparison)
            if [[ $# -lt 2 ]]; then
                echo "Error: --comparison requires an argument" >&2
                show_help
                exit 1
            fi
            COMPARISON_NAME="$2"; shift 2 ;;
        --bench)
            if [[ $# -lt 2 ]]; then
                echo "Error: --bench requires an argument" >&2
                show_help
                exit 1
            fi
            BENCH_NAME="$2"; shift 2 ;;
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

mkdir -p "$OUTPUT_DIR"

if [[ -f "scripts/benchmark-env.sh" ]]; then
    # shellcheck source=scripts/benchmark-env.sh
    source scripts/benchmark-env.sh
else
    echo "Warning: benchmark-env.sh not found, using defaults" >&2
    export RUSTFLAGS="${RUSTFLAGS:--C lto=fat -C codegen-units=1 -C opt-level=3}"
    export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/benchmark}"
fi

echo "=== Inline Experiment ==="
echo "Baseline: $BASELINE_NAME / Comparison: $COMPARISON_NAME"
echo "Benchmark: $BENCH_NAME / Output: $OUTPUT_DIR"
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
    echo "Baseline: $BASELINE_NAME"
    echo "Comparison: $COMPARISON_NAME"
    echo "Benchmark: $BENCH_NAME"
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

echo "Step 1: Running baseline benchmark..."
cargo bench --bench "$BENCH_NAME" -- --save-baseline "$BASELINE_NAME" 2>&1 | tee "$OUTPUT_DIR/${BASELINE_NAME}.log"

echo ""
echo "Step 2: Add #[inline] attributes to hot path functions and press Enter..."
read -r

echo "Step 3: Running comparison benchmark..."
cargo bench --bench "$BENCH_NAME" -- --save-baseline "$COMPARISON_NAME" --baseline "$BASELINE_NAME" 2>&1 | tee "$OUTPUT_DIR/${COMPARISON_NAME}.log"

echo ""
echo "=== Complete ==="
echo "Results: $OUTPUT_DIR"
echo "  - ${BASELINE_NAME}.log"
echo "  - ${COMPARISON_NAME}.log"
