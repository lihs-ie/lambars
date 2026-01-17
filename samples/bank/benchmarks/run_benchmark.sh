#!/bin/bash

# Bank Sample API Benchmark Script
# Usage: ./run_benchmark.sh [options]
#
# Options:
#   -t, --threads NUM     Number of threads (default: 4)
#   -c, --connections NUM Number of connections (default: 100)
#   -d, --duration SEC    Duration in seconds (default: 30)
#   -h, --help            Show this help

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_URL="${BASE_URL:-http://localhost:8081}"
THREADS="${THREADS:-4}"
CONNECTIONS="${CONNECTIONS:-100}"
DURATION="${DURATION:-30}"
RESULTS_DIR="${SCRIPT_DIR}/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULT_FILE="${RESULTS_DIR}/benchmark_${TIMESTAMP}.txt"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--threads)
            THREADS="$2"
            shift 2
            ;;
        -c|--connections)
            CONNECTIONS="$2"
            shift 2
            ;;
        -d|--duration)
            DURATION="$2"
            shift 2
            ;;
        -h|--help)
            head -20 "$0" | tail -14
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_wrk() {
    if ! command -v wrk &> /dev/null; then
        log_error "wrk is not installed"
        echo "Install with: brew install wrk (macOS) or apt install wrk (Linux)"
        exit 1
    fi
}

check_api() {
    log_info "Checking API availability at ${BASE_URL}..."
    local max_attempts=30
    local attempt=0

    while [ $attempt -lt $max_attempts ]; do
        if curl -s "${BASE_URL}/health" > /dev/null 2>&1; then
            log_success "API is available"
            return 0
        fi
        attempt=$((attempt + 1))
        sleep 1
    done

    log_error "API is not available after ${max_attempts} seconds"
    exit 1
}

create_account() {
    local name="$1"
    local balance="$2"

    local response=$(curl -s -X POST "${BASE_URL}/accounts" \
        -H "Content-Type: application/json" \
        -d "{\"owner_name\": \"${name}\", \"initial_balance\": {\"amount\": \"${balance}\", \"currency\": \"JPY\"}}")

    # Extract account_id from response JSON
    echo "$response" | sed -n 's/.*"account_id":"\([^"]*\)".*/\1/p'
}

deposit_to_account() {
    local account_id="$1"
    local amount="$2"

    curl -s -X POST "${BASE_URL}/accounts/${account_id}/deposit" \
        -H "Content-Type: application/json" \
        -H "Idempotency-Key: setup-deposit-${account_id}-$(date +%s)" \
        -d "{\"amount\": \"${amount}\", \"currency\": \"JPY\"}" > /dev/null
}

run_wrk() {
    local name="$1"
    local script="$2"
    shift 2
    local args="$@"

    log_info "Running benchmark: ${name}"
    echo ""
    echo "========================================" | tee -a "$RESULT_FILE"
    echo "Benchmark: ${name}" | tee -a "$RESULT_FILE"
    echo "Time: $(date)" | tee -a "$RESULT_FILE"
    echo "Config: ${THREADS} threads, ${CONNECTIONS} connections, ${DURATION}s" | tee -a "$RESULT_FILE"
    echo "========================================" | tee -a "$RESULT_FILE"

    wrk -t${THREADS} -c${CONNECTIONS} -d${DURATION}s \
        -s "${SCRIPT_DIR}/scripts/${script}" \
        "${BASE_URL}" -- ${args} 2>&1 | tee -a "$RESULT_FILE"

    echo "" | tee -a "$RESULT_FILE"
}

start_resource_monitor() {
    log_info "Starting resource monitor..."
    MONITOR_FILE="${RESULTS_DIR}/resources_${TIMESTAMP}.csv"
    echo "timestamp,container,cpu_percent,mem_usage,mem_limit,mem_percent,net_io,block_io" > "$MONITOR_FILE"

    (
        while true; do
            docker stats --no-stream --format "{{.Name}},{{.CPUPerc}},{{.MemUsage}},{{.MemPerc}},{{.NetIO}},{{.BlockIO}}" 2>/dev/null | \
            while read line; do
                echo "$(date +%Y-%m-%dT%H:%M:%S),$line"
            done >> "$MONITOR_FILE"
            sleep 1
        done
    ) &
    MONITOR_PID=$!
    log_info "Resource monitor started (PID: ${MONITOR_PID})"
}

stop_resource_monitor() {
    if [ -n "$MONITOR_PID" ]; then
        kill $MONITOR_PID 2>/dev/null || true
        log_info "Resource monitor stopped"
    fi
}

# Main
main() {
    log_info "Bank Sample API Benchmark"
    log_info "========================="
    echo ""

    # Setup
    check_wrk
    mkdir -p "$RESULTS_DIR"

    # Check API
    check_api

    # Start resource monitoring
    start_resource_monitor
    trap stop_resource_monitor EXIT

    # Initialize result file
    echo "Bank Sample API Benchmark Results" > "$RESULT_FILE"
    echo "==================================" >> "$RESULT_FILE"
    echo "Date: $(date)" >> "$RESULT_FILE"
    echo "Base URL: ${BASE_URL}" >> "$RESULT_FILE"
    echo "Threads: ${THREADS}" >> "$RESULT_FILE"
    echo "Connections: ${CONNECTIONS}" >> "$RESULT_FILE"
    echo "Duration: ${DURATION}s" >> "$RESULT_FILE"
    echo "" >> "$RESULT_FILE"

    # Create test accounts
    log_info "Creating test accounts..."
    ACCOUNT1=$(create_account "Benchmark User 1" 0)
    ACCOUNT2=$(create_account "Benchmark User 2" 0)
    log_success "Created accounts: ${ACCOUNT1}, ${ACCOUNT2}"

    # Deposit large amount for withdraw/transfer tests
    log_info "Depositing initial balance..."
    deposit_to_account "$ACCOUNT1" 100000000
    deposit_to_account "$ACCOUNT2" 100000000
    log_success "Initial balance deposited"

    echo "" | tee -a "$RESULT_FILE"
    echo "Test Accounts:" | tee -a "$RESULT_FILE"
    echo "  Account 1: ${ACCOUNT1}" | tee -a "$RESULT_FILE"
    echo "  Account 2: ${ACCOUNT2}" | tee -a "$RESULT_FILE"
    echo "" | tee -a "$RESULT_FILE"

    # Run benchmarks
    log_info "Starting benchmarks..."
    echo ""

    # 1. Health check (baseline)
    run_wrk "Health Check (Baseline)" "health.lua"

    # 2. Deposit (traditional)
    run_wrk "Deposit (Traditional)" "deposit.lua" "$ACCOUNT1"

    # 3. Deposit (eff_async!)
    run_wrk "Deposit (eff_async!)" "deposit.lua" "$ACCOUNT1" "eff"

    # 4. Withdraw (traditional)
    run_wrk "Withdraw (Traditional)" "withdraw.lua" "$ACCOUNT1"

    # 5. Withdraw (eff_async!)
    run_wrk "Withdraw (eff_async!)" "withdraw.lua" "$ACCOUNT1" "eff"

    # 6. Transfer
    run_wrk "Transfer" "transfer.lua" "$ACCOUNT1" "$ACCOUNT2"

    # Summary
    echo ""
    log_success "Benchmark completed!"
    echo ""
    echo "Results saved to:"
    echo "  - ${RESULT_FILE}"
    echo "  - ${MONITOR_FILE}"

    # Print resource summary
    echo ""
    echo "Resource Usage Summary:" | tee -a "$RESULT_FILE"
    echo "----------------------" | tee -a "$RESULT_FILE"
    if [ -f "$MONITOR_FILE" ]; then
        # Get max CPU and memory for bank-app
        tail -n +2 "$MONITOR_FILE" | grep "bank-app" | \
        awk -F',' '{
            gsub(/%/, "", $3);
            gsub(/%/, "", $6);
            if ($3 > max_cpu) max_cpu = $3;
            if ($6 > max_mem) max_mem = $6;
        }
        END {
            printf "  Max CPU: %.2f%%\n", max_cpu;
            printf "  Max Memory: %.2f%%\n", max_mem;
        }' | tee -a "$RESULT_FILE"
    fi
}

main
