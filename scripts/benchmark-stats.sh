#!/usr/bin/env bash
set -euo pipefail

show_help() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

ベンチマーク結果から統計情報を計算します。

OPTIONS:
    --input FILE         入力CSVファイル [required]
    --output FILE        出力ファイル [default: stdout]
    --help               ヘルプを表示

INPUT FORMAT:
    run,benchmark,instructions
    1,push_back_1000,312708
    2,push_back_1000,312800

EXAMPLES:
    scripts/benchmark-stats.sh --input results.csv
    scripts/benchmark-stats.sh --input results.csv --output stats.txt
EOF
}

INPUT_FILE=""
OUTPUT_FILE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --input)
            if [[ $# -lt 2 ]]; then
                echo "Error: --input requires an argument" >&2
                show_help
                exit 1
            fi
            INPUT_FILE="$2"; shift 2 ;;
        --output)
            if [[ $# -lt 2 ]]; then
                echo "Error: --output requires an argument" >&2
                show_help
                exit 1
            fi
            OUTPUT_FILE="$2"; shift 2 ;;
        --help) show_help; exit 0 ;;
        *) echo "Error: Unknown option: $1" >&2; show_help; exit 1 ;;
    esac
done

if [[ -z "$INPUT_FILE" ]]; then
    echo "Error: --input is required" >&2
    show_help
    exit 1
fi

if [[ ! -f "$INPUT_FILE" ]]; then
    echo "Error: Input file not found: $INPUT_FILE" >&2
    exit 1
fi

calculate_stats() {
    awk -F, '
    BEGIN { count = sum = sum_sq = 0; min = max = -1 }
    NR > 1 && $3 ~ /^[0-9]+$/ {
        count++; sum += $3; sum_sq += $3 * $3
        if (min == -1 || $3 < min) min = $3
        if (max == -1 || $3 > max) max = $3
        values[count] = $3
        run_numbers[count] = $1
    }
    END {
        if (count == 0) { print "No data found"; exit 1 }
        mean = sum / count
        variance = sum_sq / count - mean * mean
        # Clamp variance to avoid negative values due to rounding errors
        if (variance < 0) variance = 0
        stddev = sqrt(variance)
        cv = mean > 0 ? stddev / mean * 100 : 0

        print "=== Benchmark Statistics ==="
        printf "Count:     %d\nMean:      %.2f\nStd Dev:   %.2f\nCV:        %.2f%%\nMin:       %d\nMax:       %d\nRange:     %d\n\n",
               count, mean, stddev, cv, min, max, max - min

        print "=== Outlier Detection (Mean ± 2σ) ==="
        lower = mean - 2 * stddev; upper = mean + 2 * stddev
        printf "Bounds:    %.2f - %.2f\n", lower, upper
        outliers = 0
        for (i = 1; i <= count; i++) {
            if (values[i] < lower || values[i] > upper) {
                if (!outliers++) print "Outliers:"
                # Avoid division by zero when stddev == 0
                sigma_value = stddev > 0 ? (values[i] - mean) / stddev : 0
                printf "  Run %s: %d (%.2f σ)\n", run_numbers[i], values[i], sigma_value
            }
        }
        if (!outliers) print "No outliers\n"

        print "\n=== Stability ==="
        if (cv <= 3) print "✓ STABLE (CV <= 3%)"
        else if (cv <= 5) print "⚠ MARGINAL (CV <= 5%)"
        else if (cv <= 10) print "⚠ UNSTABLE (CV <= 10%)"
        else print "✗ VERY UNSTABLE (CV > 10%)"
    }
    ' "$1"
}

if [[ -n "$OUTPUT_FILE" ]]; then
    calculate_stats "$INPUT_FILE" > "$OUTPUT_FILE"
    echo "Statistics written to: $OUTPUT_FILE"
else
    calculate_stats "$INPUT_FILE"
fi
