#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCHMARK_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_DIR="$(dirname "$BENCHMARK_DIR")"

BASE_URL="${BASE_URL:-http://localhost:8080}"
SCENARIO="${1:-smoke}"

echo "=================================================="
echo "  Roguelike API Benchmark Runner"
echo "=================================================="
echo ""
echo "Scenario:  $SCENARIO"
echo "Base URL:  $BASE_URL"
echo ""

cd "$PROJECT_DIR"

if ! curl -s "$BASE_URL/api/v1/health" > /dev/null 2>&1; then
    echo "ERROR: API server is not responding at $BASE_URL"
    echo "Please start the server with: docker-compose up -d"
    exit 1
fi

echo "API server is healthy. Starting benchmark..."
echo ""

mkdir -p "$BENCHMARK_DIR/results"

k6 run \
    -e SCENARIO="$SCENARIO" \
    -e BASE_URL="$BASE_URL" \
    "$BENCHMARK_DIR/k6/main.js"

echo ""
echo "Benchmark complete. Results saved to: $BENCHMARK_DIR/results/"
