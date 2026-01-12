#!/bin/bash

CONTAINER_NAME="${1:-roguelike-app}"
INTERVAL="${2:-1}"
DURATION="${3:-60}"
OUTPUT_FILE="${4:-}"

echo "========================================"
echo "Docker Memory Monitor"
echo "========================================"
echo "Container: $CONTAINER_NAME"
echo "Interval: ${INTERVAL}s"
echo "Duration: ${DURATION}s"
echo "Started: $(date '+%Y-%m-%d %H:%M:%S')"
echo "========================================"
echo ""

if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "Error: Container '$CONTAINER_NAME' is not running"
    exit 1
fi

declare -a MEMORY_VALUES
SAMPLE_COUNT=$((DURATION / INTERVAL))

echo "Timestamp,MemUsage,MemLimit,MemPercent"
if [ -n "$OUTPUT_FILE" ]; then
    echo "Timestamp,MemUsage,MemLimit,MemPercent" > "$OUTPUT_FILE"
fi

for ((i=0; i<SAMPLE_COUNT; i++)); do
    TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
    STATS=$(docker stats "$CONTAINER_NAME" --no-stream --format "{{.MemUsage}},{{.MemPerc}}" 2>/dev/null)

    if [ -z "$STATS" ]; then
        echo "Warning: Failed to get stats for $CONTAINER_NAME"
        sleep "$INTERVAL"
        continue
    fi

    MEM_USAGE=$(echo "$STATS" | cut -d',' -f1 | cut -d'/' -f1 | tr -d ' ')
    MEM_LIMIT=$(echo "$STATS" | cut -d',' -f1 | cut -d'/' -f2 | tr -d ' ')
    MEM_PERCENT=$(echo "$STATS" | cut -d',' -f2 | tr -d '%')

    echo "$TIMESTAMP,$MEM_USAGE,$MEM_LIMIT,$MEM_PERCENT%"
    if [ -n "$OUTPUT_FILE" ]; then
        echo "$TIMESTAMP,$MEM_USAGE,$MEM_LIMIT,$MEM_PERCENT%" >> "$OUTPUT_FILE"
    fi

    MEM_BYTES=$(echo "$MEM_USAGE" | sed 's/MiB/*1048576/;s/GiB/*1073741824/;s/KiB/*1024/' | bc 2>/dev/null || echo "0")
    MEMORY_VALUES+=("$MEM_BYTES")

    sleep "$INTERVAL"
done

echo ""
echo "========================================"
echo "Summary"
echo "========================================"

if [ ${#MEMORY_VALUES[@]} -gt 0 ]; then
    MIN_MEM=${MEMORY_VALUES[0]}
    MAX_MEM=${MEMORY_VALUES[0]}
    TOTAL_MEM=0

    for mem in "${MEMORY_VALUES[@]}"; do
        TOTAL_MEM=$((TOTAL_MEM + mem))
        if [ "$mem" -lt "$MIN_MEM" ]; then
            MIN_MEM=$mem
        fi
        if [ "$mem" -gt "$MAX_MEM" ]; then
            MAX_MEM=$mem
        fi
    done

    AVG_MEM=$((TOTAL_MEM / ${#MEMORY_VALUES[@]}))

    echo "Samples: ${#MEMORY_VALUES[@]}"
    echo "Min Memory: $(echo "scale=2; $MIN_MEM / 1048576" | bc) MiB"
    echo "Max Memory: $(echo "scale=2; $MAX_MEM / 1048576" | bc) MiB"
    echo "Avg Memory: $(echo "scale=2; $AVG_MEM / 1048576" | bc) MiB"

    echo ""
    echo "SLO Check:"
    if [ "$MAX_MEM" -lt 104857600 ]; then
        echo "  Idle (<100MB): PASS"
    else
        echo "  Idle (<100MB): FAIL (Max: $(echo "scale=2; $MAX_MEM / 1048576" | bc) MiB)"
    fi

    if [ "$MAX_MEM" -lt 524288000 ]; then
        echo "  Load (<500MB): PASS"
    else
        echo "  Load (<500MB): FAIL (Max: $(echo "scale=2; $MAX_MEM / 1048576" | bc) MiB)"
    fi
fi

echo "========================================"
echo "Finished: $(date '+%Y-%m-%d %H:%M:%S')"
echo "========================================"
