#!/usr/bin/env bash
set -euo pipefail

REDIS_CLI="${REDIS_CLI:-redis-cli}"

# Default cache file path tracker (user-specific for security)
CACHE_FILE_TRACKER="${TMPDIR:-/tmp}/cache_stats_last_file_${USER:-$(id -u)}.txt"

# Store whether CACHE_STATS_FILE was explicitly set
CACHE_STATS_FILE_EXPLICIT=false
if [[ -n "${CACHE_STATS_FILE:-}" ]]; then
    CACHE_STATS_FILE_EXPLICIT=true
fi

check_redis() {
    if ! command -v "$REDIS_CLI" &> /dev/null; then
        >&2 echo "Error: redis-cli not found"
        exit 1
    fi
    if ! "$REDIS_CLI" ping &> /dev/null; then
        >&2 echo "Error: Redis server is not available"
        exit 1
    fi
}

get_keyspace_stats() {
    local stats=$("$REDIS_CLI" INFO stats)
    local hits=$(echo "$stats" | grep -E "^keyspace_hits:" | cut -d: -f2 | tr -d '\r\n' || echo "0")
    local misses=$(echo "$stats" | grep -E "^keyspace_misses:" | cut -d: -f2 | tr -d '\r\n' || echo "0")
    echo "$hits $misses"
}

cmd_before() {
    check_redis

    # Create temporary file if not explicitly set
    if [[ "$CACHE_STATS_FILE_EXPLICIT" == "false" ]]; then
        CACHE_STATS_FILE=$(mktemp -t cache_stats.XXXXXX)
        # Store the path atomically using a temporary file
        local tracker_tmp=$(mktemp -t cache_tracker.XXXXXX)
        echo "$CACHE_STATS_FILE" > "$tracker_tmp"
        mv "$tracker_tmp" "$CACHE_FILE_TRACKER"
        >&2 echo "Using temporary file: $CACHE_STATS_FILE"
    fi

    get_keyspace_stats > "$CACHE_STATS_FILE"
    >&2 echo "Cache stats saved to: $CACHE_STATS_FILE"
    if [[ "$CACHE_STATS_FILE_EXPLICIT" == "true" ]]; then
        >&2 echo "To retrieve results, run: CACHE_STATS_FILE=\"$CACHE_STATS_FILE\" $0 after"
    else
        >&2 echo "To retrieve results, run: $0 after"
    fi
}

cmd_after() {
    check_redis

    # If CACHE_STATS_FILE was not explicitly set, try to read from the tracker
    if [[ "$CACHE_STATS_FILE_EXPLICIT" == "false" ]]; then
        if [[ ! -f "$CACHE_FILE_TRACKER" ]]; then
            >&2 echo "Error: No previous 'before' execution found."
            >&2 echo "Run '$0 before' first, or set CACHE_STATS_FILE explicitly."
            exit 1
        fi
        CACHE_STATS_FILE=$(cat "$CACHE_FILE_TRACKER")
        >&2 echo "Using cached file path: $CACHE_STATS_FILE"
    fi

    if [[ ! -f "$CACHE_STATS_FILE" ]]; then
        >&2 echo "Error: $CACHE_STATS_FILE not found."
        >&2 echo "The file may have been deleted. Run '$0 before' again."
        exit 1
    fi

    read -r before_hits before_misses < "$CACHE_STATS_FILE"
    read -r after_hits after_misses <<< "$(get_keyspace_stats)"

    local hits=$((after_hits - before_hits))
    local misses=$((after_misses - before_misses))

    # Validate differences: negative values indicate Redis stats reset
    if [[ $hits -lt 0 ]]; then
        >&2 echo "Warning: Negative hits difference detected ($hits). Redis stats may have been reset. Using 0."
        hits=0
    fi
    if [[ $misses -lt 0 ]]; then
        >&2 echo "Warning: Negative misses difference detected ($misses). Redis stats may have been reset. Using 0."
        misses=0
    fi

    local total=$((hits + misses))
    local hit_rate=$(awk -v h="$hits" -v t="$total" 'BEGIN {printf "%.4f", (t > 0 ? h / t : 0)}')

    cat <<EOF
{
  "hits": $hits,
  "misses": $misses,
  "total": $total,
  "hit_rate": $hit_rate
}
EOF

    # Clean up the stats file and the tracker (only if not explicitly set)
    rm -f "$CACHE_STATS_FILE"
    if [[ "$CACHE_STATS_FILE_EXPLICIT" == "false" ]]; then
        rm -f "$CACHE_FILE_TRACKER"
    fi
}

case "${1:-}" in
    before) cmd_before ;;
    after)  cmd_after ;;
    *)
        >&2 echo "Usage: $0 {before|after}"
        exit 1
        ;;
esac
