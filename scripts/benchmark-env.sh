#!/usr/bin/env bash
set -euo pipefail

show_help() {
    cat <<EOF
Usage: source $(basename "${BASH_SOURCE[0]}") [OPTIONS]

環境変数とビルド設定を固定するスクリプト。source コマンドで実行してください。

OPTIONS:
    --lto LTO_MODE       LTO モード (fat|thin|off) [default: fat]
    --cgu UNITS          codegen-units (1-256) [default: 1]
    --help               ヘルプを表示

EXAMPLES:
    source scripts/benchmark-env.sh
    source scripts/benchmark-env.sh --lto thin --cgu 16
EOF
}

LTO_MODE="fat"
CGU_UNITS="1"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --lto)
            if [[ $# -lt 2 ]]; then
                echo "Error: --lto requires an argument" >&2
                show_help
                return 1 2>/dev/null || exit 1
            fi
            LTO_MODE="$2"; shift 2 ;;
        --cgu)
            if [[ $# -lt 2 ]]; then
                echo "Error: --cgu requires an argument" >&2
                show_help
                return 1 2>/dev/null || exit 1
            fi
            CGU_UNITS="$2"; shift 2 ;;
        --help) show_help; return 0 2>/dev/null || exit 0 ;;
        *) echo "Error: Unknown option: $1" >&2; show_help; return 1 2>/dev/null || exit 1 ;;
    esac
done

case "$LTO_MODE" in
    fat|thin|off) ;;
    *) echo "Error: Invalid LTO mode: $LTO_MODE (must be fat|thin|off)" >&2; return 1 2>/dev/null || exit 1 ;;
esac

if ! [[ "$CGU_UNITS" =~ ^[0-9]+$ ]] || [ "$CGU_UNITS" -lt 1 ] || [ "$CGU_UNITS" -gt 256 ]; then
    echo "Error: Invalid codegen-units: $CGU_UNITS (must be 1-256)" >&2
    return 1 2>/dev/null || exit 1
fi

echo "rustc version: $(rustc --version)"
export RUSTFLAGS="-C lto=$LTO_MODE -C codegen-units=$CGU_UNITS -C opt-level=3"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/benchmark}"
export BENCH_LTO="$LTO_MODE"
export BENCH_CGU="$CGU_UNITS"

echo "RUSTFLAGS=$RUSTFLAGS"
echo "CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
echo ""
echo "Environment configured. Run: cargo bench --bench <benchmark_name>"
