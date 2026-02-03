#!/usr/bin/env bash
set -euo pipefail

tmp_file=$(mktemp)
trap 'rm -f "$tmp_file"' EXIT
cat > "$tmp_file"

convert_to_ms() {
    local value="$1"
    [[ -z "$value" ]] && return

    if [[ "$value" =~ ^([0-9.]+)([a-z]+)$ ]]; then
        local num="${BASH_REMATCH[1]}"
        local unit="${BASH_REMATCH[2]}"

        case "$unit" in
            us) awk -v n="$num" 'BEGIN {printf "%.2f", n / 1000}' ;;
            ms) awk -v n="$num" 'BEGIN {printf "%.2f", n}' ;;
            s)  awk -v n="$num" 'BEGIN {printf "%.2f", n * 1000}' ;;
            *)  >&2 echo "Warning: Unknown unit: $unit" ;;
        esac
    fi
}

extract_percentile() {
    convert_to_ms "$(grep -E "^[[:space:]]*$1%" "$tmp_file" | awk '{print $2}' || true)"
}

safe_number() {
    local value="$1"
    [[ -z "$value" ]] && echo "null" && return

    # Check for nan/inf (case-insensitive)
    if [[ "$value" =~ ^[Nn][Aa][Nn]$ ]] || [[ "$value" =~ ^[Ii][Nn][Ff]$ ]] || [[ "$value" =~ ^[+-]?[Ii][Nn][Ff]$ ]]; then
        echo "null"
    else
        echo "$value"
    fi
}

rps_raw=$(grep -E "Requests/sec:" "$tmp_file" | awk '{print $2}' || true)
rps=$(safe_number "$rps_raw")

latency_line=$(grep -E "^[[:space:]]*Latency" "$tmp_file" | head -n1 || true)
if [[ -n "$latency_line" ]]; then
    latency_mean_ms=$(convert_to_ms "$(echo "$latency_line" | awk '{print $2}')")
    latency_stdev_ms=$(convert_to_ms "$(echo "$latency_line" | awk '{print $3}')")
    latency_max_ms=$(convert_to_ms "$(echo "$latency_line" | awk '{print $4}')")
fi

p50_ms=$(extract_percentile 50)
p75_ms=$(extract_percentile 75)
p90_ms=$(extract_percentile 90)
p95_ms=$(extract_percentile 95)
p99_ms=$(extract_percentile 99)

cat <<EOF
{
  "rps": $rps,
  "latency_mean_ms": ${latency_mean_ms:-null},
  "latency_stdev_ms": ${latency_stdev_ms:-null},
  "latency_max_ms": ${latency_max_ms:-null},
  "p50_latency_ms": ${p50_ms:-null},
  "p75_latency_ms": ${p75_ms:-null},
  "p90_latency_ms": ${p90_ms:-null},
  "p95_latency_ms": ${p95_ms:-null},
  "p99_latency_ms": ${p99_ms:-null}
}
EOF
