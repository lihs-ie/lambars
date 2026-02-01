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

LTO/CGU 設定の影響を検証するスクリプト。
lto=fat/thin/off と codegen-units=1/16 の組み合わせで比較します。

OPTIONS:
    --bench NAME         ベンチマーク名 [default: persistent_vector]
    --output DIR         出力ディレクトリ [default: benches/results/lto-cgu-experiment]
    --help               ヘルプを表示

EXAMPLES:
    scripts/lto-cgu-experiment.sh
    scripts/lto-cgu-experiment.sh --bench my_bench --output /tmp/results
EOF
}

BENCH_NAME="persistent_vector"
OUTPUT_DIR="benches/results/lto-cgu-experiment"

while [[ $# -gt 0 ]]; do
    case "$1" in
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

echo "=== LTO/CGU Experiment ==="
echo "Benchmark: $BENCH_NAME / Output: $OUTPUT_DIR"
echo ""

declare -a LTO_MODES=("fat" "thin" "off")
declare -a CGU_VALUES=("1" "16")

METADATA_FILE="$OUTPUT_DIR/metadata.txt"
{
    echo "=== Experiment Metadata ==="
    echo "Date: $(date -u +"%Y-%m-%d %H:%M:%S UTC")"
    echo "Rustc Version: $(rustc --version)"
    echo "CPU: $(get_cpu_info)"
    echo "OS: $(uname -s) $(uname -r)"
    echo "Benchmark: $BENCH_NAME"
    echo "LTO Modes: ${LTO_MODES[*]}"
    echo "CGU Values: ${CGU_VALUES[*]}"
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

for lto in "${LTO_MODES[@]}"; do
    for cgu in "${CGU_VALUES[@]}"; do
        BASELINE_NAME="lto-${lto}-cgu-${cgu}"
        LOG_FILE="$OUTPUT_DIR/${BASELINE_NAME}.log"
        RUN_METADATA_FILE="$OUTPUT_DIR/${BASELINE_NAME}-metadata.txt"

        echo "=== lto=$lto, codegen-units=$cgu ==="
        export RUSTFLAGS="-C lto=$lto -C codegen-units=$cgu -C opt-level=3"
        export CARGO_TARGET_DIR="target/benchmark-lto-cgu"

        # Save run-specific metadata
        {
            echo "=== Run Metadata: $BASELINE_NAME ==="
            echo "Date: $(date -u +"%Y-%m-%d %H:%M:%S UTC")"
            echo "RUSTFLAGS: $RUSTFLAGS"
            echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"
        } > "$RUN_METADATA_FILE"

        cargo clean
        if cargo bench --bench "$BENCH_NAME" -- --save-baseline "$BASELINE_NAME" 2>&1 | tee "$LOG_FILE"; then
            echo "✓ $BASELINE_NAME"
        else
            echo "✗ $BASELINE_NAME" >&2
        fi
        echo ""
    done
done

SUMMARY_FILE="$OUTPUT_DIR/summary.txt"
{
    echo "=== LTO/CGU Experiment Summary ==="
    echo "Benchmark: $BENCH_NAME"
    echo "Generated: $(date)"
    echo ""
    for lto in "${LTO_MODES[@]}"; do
        for cgu in "${CGU_VALUES[@]}"; do
            BASELINE_NAME="lto-${lto}-cgu-${cgu}"
            LOG_FILE="$OUTPUT_DIR/${BASELINE_NAME}.log"
            echo "lto=$lto, codegen-units=$cgu"
            if [[ -f "$LOG_FILE" ]] && grep -q "push_back_1000" "$LOG_FILE"; then
                # Extract instructions from the line after "push_back_1000" (IAI Callgrind format)
                # Use || true to continue on format mismatch
                INSTRUCTIONS=$(grep -A 1 "push_back_1000" "$LOG_FILE" | grep "Instructions:" | grep -o '[0-9,]\+' | tr -d ',' | head -1 || true)
                if [[ -n "$INSTRUCTIONS" ]]; then
                    echo "  push_back_1000 Instructions: $INSTRUCTIONS"
                else
                    echo "  (no instructions data)"
                fi
            else
                echo "  (no data)"
            fi
            echo ""
        done
    done
} > "$SUMMARY_FILE"

echo "=== Complete ==="
echo "Summary: $SUMMARY_FILE"
