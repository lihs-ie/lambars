#!/bin/bash
# benches/api/benchmarks/run_benchmark.sh
#
# Run wrk benchmarks for Phase 2 API endpoints
#
# Usage:
#   ./run_benchmark.sh                    # Run all benchmarks
#   ./run_benchmark.sh phase2_misc        # Run specific benchmark
#   ./run_benchmark.sh --quick            # Quick test (5s duration)

set -euo pipefail

# Configuration
API_URL="${API_URL:-http://localhost:3002}"
THREADS="${THREADS:-2}"
CONNECTIONS="${CONNECTIONS:-10}"
DURATION="${DURATION:-30s}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="${SCRIPT_DIR}/results/$(date +%Y%m%d_%H%M%S)"

# Parse arguments
QUICK_MODE=false
SPECIFIC_SCRIPT=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            QUICK_MODE=true
            DURATION="5s"
            shift
            ;;
        *)
            SPECIFIC_SCRIPT="$1"
            shift
            ;;
    esac
done

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=============================================="
echo "  API Workload Benchmark - Phase 2 Endpoints"
echo "=============================================="
echo ""
echo "Configuration:"
echo "  API URL:     ${API_URL}"
echo "  Threads:     ${THREADS}"
echo "  Connections: ${CONNECTIONS}"
echo "  Duration:    ${DURATION}"
echo "  Results:     ${RESULTS_DIR}"
echo ""

# Check if wrk is installed
if ! command -v wrk &> /dev/null; then
    echo -e "${RED}Error: wrk is not installed${NC}"
    echo "Install with:"
    echo "  macOS:  brew install wrk"
    echo "  Ubuntu: apt-get install wrk"
    exit 1
fi

# Health check
echo -n "Checking API health... "
if curl -sf "${API_URL}/health" > /dev/null 2>&1; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo "API is not responding at ${API_URL}/health"
    echo ""
    echo "Start the API server with:"
    echo "  cd benches/api/docker && docker compose up -d"
    echo "  # or"
    echo "  cargo run -p task-management-benchmark-api"
    exit 1
fi

# Create results directory
mkdir -p "${RESULTS_DIR}"

# Summary file
SUMMARY_FILE="${RESULTS_DIR}/summary.txt"
echo "Benchmark Results - $(date)" > "${SUMMARY_FILE}"
echo "================================" >> "${SUMMARY_FILE}"
echo "" >> "${SUMMARY_FILE}"

# Run benchmarks
run_benchmark() {
    local script_name="$1"
    local script_path="${SCRIPT_DIR}/scripts/${script_name}.lua"

    if [[ ! -f "${script_path}" ]]; then
        echo -e "${YELLOW}Warning: Script not found: ${script_path}${NC}"
        return 1
    fi

    echo ""
    echo "----------------------------------------------"
    echo "Running: ${script_name}"
    echo "----------------------------------------------"

    local result_file="${RESULTS_DIR}/${script_name}.txt"

    # Run wrk and capture output
    cd "${SCRIPT_DIR}"
    if wrk -t"${THREADS}" -c"${CONNECTIONS}" -d"${DURATION}" \
        --script="scripts/${script_name}.lua" \
        "${API_URL}" 2>&1 | tee "${result_file}"; then

        # Extract key metrics for summary
        local reqs_sec=$(grep "Requests/sec:" "${result_file}" | awk '{print $2}')
        local avg_latency=$(grep "Latency" "${result_file}" | head -1 | awk '{print $2}')

        echo "" >> "${SUMMARY_FILE}"
        echo "${script_name}:" >> "${SUMMARY_FILE}"
        echo "  Requests/sec: ${reqs_sec:-N/A}" >> "${SUMMARY_FILE}"
        echo "  Avg Latency:  ${avg_latency:-N/A}" >> "${SUMMARY_FILE}"

        echo -e "${GREEN}Completed${NC}"
    else
        echo -e "${RED}Failed${NC}"
        echo "${script_name}: FAILED" >> "${SUMMARY_FILE}"
    fi
}

# Get list of scripts to run
if [[ -n "${SPECIFIC_SCRIPT}" ]]; then
    SCRIPTS=("${SPECIFIC_SCRIPT}")
else
    SCRIPTS=(
        "phase2_recursive"
        "phase2_ordered"
        "phase2_traversable"
        "phase2_alternative"
        "phase2_async_pipeline"
        "phase2_bifunctor"
        "phase2_applicative"
        "phase2_optics"
        "phase2_misc"
    )
fi

# Run all benchmarks
for script in "${SCRIPTS[@]}"; do
    run_benchmark "${script}" || true
done

echo ""
echo "=============================================="
echo "  Benchmark Complete"
echo "=============================================="
echo ""
echo "Results saved to: ${RESULTS_DIR}"
echo ""
echo "Summary:"
cat "${SUMMARY_FILE}"
