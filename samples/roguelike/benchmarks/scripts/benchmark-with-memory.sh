#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
RESULTS_DIR="$PROJECT_DIR/benchmarks/results"
K6_DIR="$PROJECT_DIR/benchmarks/k6"

SCENARIO="${1:-smoke}"
CONTAINER="${2:-roguelike-app}"

mkdir -p "$RESULTS_DIR"

TIMESTAMP=$(date '+%Y%m%d_%H%M%S')
MEMORY_LOG="$RESULTS_DIR/memory_${SCENARIO}_${TIMESTAMP}.csv"

echo "========================================"
echo "Benchmark with Memory Monitoring"
echo "========================================"
echo "Scenario: $SCENARIO"
echo "Container: $CONTAINER"
echo "Results: $RESULTS_DIR"
echo "========================================"
echo ""

if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER}$"; then
    echo "Error: Container '$CONTAINER' is not running"
    echo "Please start Docker environment first:"
    echo "  docker-compose up -d"
    exit 1
fi

if ! command -v k6 &> /dev/null; then
    echo "Error: k6 is not installed"
    echo "Please install k6: https://k6.io/docs/getting-started/installation/"
    exit 1
fi

echo "Starting baseline memory measurement..."
BASELINE=$(docker stats "$CONTAINER" --no-stream --format "{{.MemUsage}}" | cut -d'/' -f1 | tr -d ' ')
echo "Baseline memory: $BASELINE"
echo ""

case "$SCENARIO" in
    smoke)
        DURATION=35
        ;;
    load)
        DURATION=510
        ;;
    stress)
        DURATION=900
        ;;
    *)
        DURATION=60
        ;;
esac

echo "Starting memory monitoring (duration: ${DURATION}s)..."
"$SCRIPT_DIR/monitor-memory.sh" "$CONTAINER" 2 "$DURATION" "$MEMORY_LOG" &
MONITOR_PID=$!

sleep 2

echo ""
echo "Starting k6 benchmark..."
echo "========================================"

cd "$PROJECT_DIR" || exit 1
k6 run -e SCENARIO="$SCENARIO" "$K6_DIR/main.js"

K6_EXIT_CODE=$?

echo ""
echo "Waiting for memory monitoring to complete..."
wait $MONITOR_PID 2>/dev/null

echo ""
echo "========================================"
echo "Results"
echo "========================================"
echo "k6 exit code: $K6_EXIT_CODE"
echo "Memory log: $MEMORY_LOG"

if [ -f "$MEMORY_LOG" ]; then
    echo ""
    echo "Memory Statistics:"
    tail -10 "$MEMORY_LOG"
fi

PEAK=$(docker stats "$CONTAINER" --no-stream --format "{{.MemUsage}}" | cut -d'/' -f1 | tr -d ' ')
echo ""
echo "Final memory: $PEAK"

echo "========================================"
echo "Benchmark Complete"
echo "========================================"
