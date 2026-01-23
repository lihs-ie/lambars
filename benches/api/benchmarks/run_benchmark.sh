#!/bin/bash
# benches/api/benchmarks/run_benchmark.sh
#
# Run wrk benchmarks for API endpoints
#
# Usage:
#   ./run_benchmark.sh                    # Run all benchmarks
#   ./run_benchmark.sh misc               # Run specific benchmark
#   ./run_benchmark.sh --quick            # Quick test (5s duration)
#   ./run_benchmark.sh --scenario <yaml>  # Run with scenario configuration
#   ./run_benchmark.sh --scenario <yaml> --quick misc  # Combined options
#   ./run_benchmark.sh --profile          # Run with perf profiling
#   ./run_benchmark.sh --scenario <yaml> --profile  # Scenario with profiling
#
# Environment Variables (set via scenario YAML or directly):
#   API_URL          - API server URL (default: http://localhost:3002)
#   STORAGE_MODE     - in_memory | postgres
#   CACHE_MODE       - in_memory | redis
#   DATA_SCALE       - 1e2 | 1e4 | 1e6 (maps from small/medium/large)
#   HIT_RATE         - 0 | 50 | 90
#   CACHE_STRATEGY   - read-through | write-through | write-behind
#   RPS_PROFILE      - steady | ramp | burst
#   THREADS          - wrk threads
#   CONNECTIONS      - wrk connections
#   DURATION         - wrk duration
#   POOL_SIZES       - DB+Redis pool size (combined)
#   WORKERS          - worker threads
#   FAIL_RATE        - 0 | 0.1 | 0.5
#   RETRY            - true | false
#   PROFILE          - true | false
#   ENDPOINT         - target endpoint
#   PAYLOAD          - small | medium | large

set -euo pipefail

# Configuration
API_URL="${API_URL:-http://localhost:3002}"
THREADS="${THREADS:-2}"
CONNECTIONS="${CONNECTIONS:-10}"
DURATION="${DURATION:-30s}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
RESULTS_DIR="${SCRIPT_DIR}/results/${TIMESTAMP}"

# Scenario configuration file
SCENARIO_FILE=""
SCENARIO_NAME=""

# Parse arguments
QUICK_MODE=false
PROFILE_MODE=false
SPECIFIC_SCRIPT=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            QUICK_MODE=true
            DURATION="5s"
            shift
            ;;
        --scenario)
            SCENARIO_FILE="$2"
            shift 2
            ;;
        --profile)
            PROFILE_MODE=true
            export PROFILE="true"
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

# =============================================================================
# Scenario YAML Environment Variable Loading
# =============================================================================
#
# If a scenario file is provided, extract environment variables from it.
# Maps scenario YAML keys to the required environment variables.
#
# Requires: yq (https://github.com/mikefarah/yq)
#
# Environment Variable Mapping:
#   YAML Key                          -> Environment Variable
#   --------------------------------------------------------
#   name                              -> SCENARIO_NAME
#   storage_mode                      -> STORAGE_MODE
#   cache_mode                        -> CACHE_MODE
#   data_scale (small/medium/large)   -> DATA_SCALE (1e2/1e4/1e6)
#   payload_variant or metadata.payload -> PAYLOAD (small/medium/large)
#   rps_profile (constant/ramp_up_down/burst) -> RPS_PROFILE (steady/ramp/burst)
#   threads                           -> THREADS
#   connections                       -> CONNECTIONS
#   duration_seconds                  -> DURATION
#   concurrency.worker_threads        -> WORKERS
#   concurrency.database_pool_size + redis_pool_size -> POOL_SIZES
#   cache_metrics.expected_hit_rate or metadata.hit_rate -> HIT_RATE (0/50/90)
#   metadata.cache_strategy           -> CACHE_STRATEGY
#   error_config.inject_error_rate or metadata.fail_injection -> FAIL_RATE
#   error_config.max_retries > 0 or metadata.retry -> RETRY
#   profiling.enable_perf or metadata.profile -> PROFILE
#   endpoints[0] or metadata.endpoint -> ENDPOINT
# =============================================================================

load_scenario_env_vars() {
    local scenario_file="$1"

    if [[ ! -f "${scenario_file}" ]]; then
        echo -e "${RED}Error: Scenario file not found: ${scenario_file}${NC}"
        exit 1
    fi

    # Check if yq is installed
    if ! command -v yq &> /dev/null; then
        echo -e "${YELLOW}Warning: yq is not installed. Scenario environment variables will not be loaded.${NC}"
        echo "Install with:"
        echo "  macOS:  brew install yq"
        echo "  Ubuntu: snap install yq"
        return 0
    fi

    echo "Loading scenario configuration from: ${scenario_file}"

    # ==========================================================================
    # Core Configuration
    # ==========================================================================

    # Scenario name
    SCENARIO_NAME=$(yq '.name // "benchmark"' "${scenario_file}" | tr -d '"')
    export SCENARIO_NAME

    # Storage mode
    local storage_mode
    storage_mode=$(yq '.storage_mode // "in_memory"' "${scenario_file}" | tr -d '"')
    export STORAGE_MODE="${storage_mode}"

    # Cache mode
    local cache_mode
    cache_mode=$(yq '.cache_mode // "in_memory"' "${scenario_file}" | tr -d '"')
    export CACHE_MODE="${cache_mode}"

    # Data scale: small -> 1e2, medium -> 1e4, large -> 1e6
    local data_scale
    data_scale=$(yq '.data_scale // "medium"' "${scenario_file}" | tr -d '"')
    case "${data_scale}" in
        "small")  export DATA_SCALE="1e2" ;;
        "medium") export DATA_SCALE="1e4" ;;
        "large")  export DATA_SCALE="1e6" ;;
        *)        export DATA_SCALE="1e4" ;;
    esac

    # Payload: prefer metadata.payload, fallback to payload_variant mapping
    local payload
    payload=$(yq '.metadata.payload // null' "${scenario_file}" | tr -d '"')
    if [[ "${payload}" == "null" ]]; then
        local payload_variant
        payload_variant=$(yq '.payload_variant // "standard"' "${scenario_file}" | tr -d '"')
        case "${payload_variant}" in
            "minimal")  payload="small" ;;
            "standard") payload="medium" ;;
            "complex"|"heavy") payload="large" ;;
            *)          payload="medium" ;;
        esac
    fi
    export PAYLOAD="${payload}"

    # RPS profile: constant -> steady, ramp_up_down -> ramp, burst -> burst
    local rps_profile
    rps_profile=$(yq '.rps_profile // "constant"' "${scenario_file}" | tr -d '"')
    case "${rps_profile}" in
        "constant")    export RPS_PROFILE="steady" ;;
        "ramp_up_down") export RPS_PROFILE="ramp" ;;
        "burst")       export RPS_PROFILE="burst" ;;
        "step_up")     export RPS_PROFILE="steady" ;;
        *)             export RPS_PROFILE="steady" ;;
    esac

    # ==========================================================================
    # Load Generation Parameters
    # ==========================================================================

    # Duration
    local scenario_duration
    scenario_duration=$(yq '.duration_seconds // null' "${scenario_file}")
    if [[ "${scenario_duration}" != "null" ]]; then
        DURATION="${scenario_duration}s"
        export DURATION_SECONDS="${scenario_duration}"
    fi

    # Connections
    local scenario_connections
    scenario_connections=$(yq '.connections // null' "${scenario_file}")
    if [[ "${scenario_connections}" != "null" ]]; then
        CONNECTIONS="${scenario_connections}"
        export CONNECTIONS
    fi

    # Threads
    local scenario_threads
    scenario_threads=$(yq '.threads // null' "${scenario_file}")
    if [[ "${scenario_threads}" != "null" ]]; then
        THREADS="${scenario_threads}"
        export THREADS
    fi

    # ==========================================================================
    # Concurrency Settings (WORKERS, POOL_SIZES)
    # ==========================================================================

    # Workers
    local worker_threads
    worker_threads=$(yq '.concurrency.worker_threads // null' "${scenario_file}")
    if [[ "${worker_threads}" != "null" ]]; then
        export WORKERS="${worker_threads}"
        export WORKER_THREADS="${worker_threads}"
    fi

    # Pool sizes (combined DB + Redis)
    local database_pool_size redis_pool_size
    database_pool_size=$(yq '.concurrency.database_pool_size // 0' "${scenario_file}")
    redis_pool_size=$(yq '.concurrency.redis_pool_size // 0' "${scenario_file}")
    if [[ "${database_pool_size}" != "0" ]] || [[ "${redis_pool_size}" != "0" ]]; then
        local pool_sizes=$((database_pool_size + redis_pool_size))
        export POOL_SIZES="${pool_sizes}"
        export DATABASE_POOL_SIZE="${database_pool_size}"
        export REDIS_POOL_SIZE="${redis_pool_size}"
    fi

    local max_connections
    max_connections=$(yq '.concurrency.max_connections // null' "${scenario_file}")
    if [[ "${max_connections}" != "null" ]]; then
        export MAX_CONNECTIONS="${max_connections}"
    fi

    # ==========================================================================
    # Cache Configuration (HIT_RATE, CACHE_STRATEGY)
    # ==========================================================================

    # Hit rate: prefer metadata.hit_rate, fallback to cache_metrics.expected_hit_rate
    local hit_rate
    hit_rate=$(yq '.metadata.hit_rate // null' "${scenario_file}")
    if [[ "${hit_rate}" == "null" ]]; then
        local expected_hit_rate
        expected_hit_rate=$(yq '.cache_metrics.expected_hit_rate // null' "${scenario_file}")
        if [[ "${expected_hit_rate}" != "null" ]]; then
            # Convert 0.0-1.0 to 0/50/90
            hit_rate=$(echo "${expected_hit_rate}" | awk '{
                if ($1 <= 0.1) print 0;
                else if ($1 <= 0.6) print 50;
                else print 90;
            }')
        else
            hit_rate="50"
        fi
    fi
    export HIT_RATE="${hit_rate}"

    # Cache strategy
    local cache_strategy
    cache_strategy=$(yq '.metadata.cache_strategy // "read-through"' "${scenario_file}" | tr -d '"')
    export CACHE_STRATEGY="${cache_strategy}"

    # ==========================================================================
    # Error Configuration (FAIL_RATE, RETRY)
    # ==========================================================================

    # Fail injection rate: prefer metadata.fail_injection, fallback to error_config.inject_error_rate
    local fail_injection
    fail_injection=$(yq '.metadata.fail_injection // null' "${scenario_file}")
    if [[ "${fail_injection}" == "null" ]]; then
        fail_injection=$(yq '.error_config.inject_error_rate // 0' "${scenario_file}")
    fi
    export FAIL_RATE="${fail_injection}"

    # Retry: prefer metadata.retry, fallback to error_config.max_retries > 0
    local retry
    retry=$(yq '.metadata.retry // null' "${scenario_file}" | tr -d '"')
    if [[ "${retry}" == "null" ]]; then
        local max_retries
        max_retries=$(yq '.error_config.max_retries // 0' "${scenario_file}")
        if [[ "${max_retries}" -gt 0 ]]; then
            retry="true"
        else
            retry="false"
        fi
    fi
    export RETRY="${retry}"

    # ==========================================================================
    # Profiling Configuration
    # ==========================================================================

    # Profile: prefer metadata.profile, fallback to profiling.enable_perf
    local profile_flag
    profile_flag=$(yq '.metadata.profile // null' "${scenario_file}" | tr -d '"')
    if [[ "${profile_flag}" == "null" ]]; then
        profile_flag=$(yq '.profiling.enable_perf // false' "${scenario_file}" | tr -d '"')
    fi
    if [[ "${profile_flag}" == "true" ]]; then
        PROFILE_MODE=true
        export PROFILE="true"
    fi

    # ==========================================================================
    # Endpoint Configuration
    # ==========================================================================

    # Endpoint: prefer metadata.endpoint, fallback to endpoints[0]
    local endpoint
    endpoint=$(yq '.metadata.endpoint // null' "${scenario_file}" | tr -d '"')
    if [[ "${endpoint}" == "null" ]]; then
        endpoint=$(yq '.endpoints[0] // null' "${scenario_file}" | tr -d '"')
    fi
    if [[ "${endpoint}" != "null" ]]; then
        export ENDPOINT="${endpoint}"
    fi

    # ==========================================================================
    # Legacy Contention Level (for compatibility)
    # ==========================================================================

    local contention_level
    contention_level=$(yq '.contention_level // "low"' "${scenario_file}" | tr -d '"')
    export CONTENTION_LEVEL="${contention_level}"

    case "${contention_level}" in
        "low")    export WRITE_RATIO="10"; export TARGET_RESOURCES="1000" ;;
        "medium") export WRITE_RATIO="50"; export TARGET_RESOURCES="100" ;;
        "high")   export WRITE_RATIO="90"; export TARGET_RESOURCES="10" ;;
        *)        export WRITE_RATIO="50"; export TARGET_RESOURCES="100" ;;
    esac

    # Target RPS
    local target_rps
    target_rps=$(yq '.target_rps // 0' "${scenario_file}")
    if [[ "${target_rps}" != "0" && "${target_rps}" != "null" ]]; then
        export TARGET_RPS="${target_rps}"
    fi

    # ==========================================================================
    # Summary Output
    # ==========================================================================

    echo "  Scenario: ${SCENARIO_NAME}"
    echo "  Storage: ${STORAGE_MODE}, Cache: ${CACHE_MODE}"
    echo "  Data scale: ${DATA_SCALE}, Payload: ${PAYLOAD}"
    echo "  RPS profile: ${RPS_PROFILE}, Hit rate: ${HIT_RATE}%"
    echo "  Cache strategy: ${CACHE_STRATEGY}"
    echo "  Fail rate: ${FAIL_RATE}, Retry: ${RETRY}"
    [[ -n "${ENDPOINT:-}" ]] && echo "  Endpoint: ${ENDPOINT}"
    [[ -n "${WORKERS:-}" ]] && echo "  Workers: ${WORKERS}"
    [[ -n "${POOL_SIZES:-}" ]] && echo "  Pool sizes: ${POOL_SIZES}"
    [[ "${PROFILE_MODE}" == "true" ]] && echo "  Profiling: enabled"
}

# Load scenario environment variables if scenario file is provided
if [[ -n "${SCENARIO_FILE}" ]]; then
    load_scenario_env_vars "${SCENARIO_FILE}"
    # Update results directory to include scenario name
    RESULTS_DIR="${SCRIPT_DIR}/results/${TIMESTAMP}/${SCENARIO_NAME}"
    echo ""
fi

echo "=============================================="
echo "  API Workload Benchmark"
echo "=============================================="
echo ""
echo "Configuration:"
echo "  API URL:     ${API_URL}"
echo "  Threads:     ${THREADS}"
echo "  Connections: ${CONNECTIONS}"
echo "  Duration:    ${DURATION}"
echo "  Results:     ${RESULTS_DIR}"
[[ "${PROFILE_MODE}" == "true" ]] && echo "  Profiling:   enabled"
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

# =============================================================================
# Generate meta.json
# =============================================================================

generate_meta_json() {
    local result_file="$1"
    local script_name="$2"
    local meta_file="${RESULTS_DIR}/meta.json"

    # Parse wrk output for metrics
    local rps avg_latency p50 p95 p99 error_rate total_requests
    rps=$(grep "Requests/sec:" "${result_file}" 2>/dev/null | awk '{print $2}' || echo "0")
    avg_latency=$(grep "Latency" "${result_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "0")
    p50=$(grep "50%" "${result_file}" 2>/dev/null | awk '{print $2}' || echo "0")
    p95=$(grep "95%" "${result_file}" 2>/dev/null | awk '{print $2}' || echo "0")
    p99=$(grep "99%" "${result_file}" 2>/dev/null | awk '{print $2}' || echo "0")
    total_requests=$(grep "requests in" "${result_file}" 2>/dev/null | awk '{print $1}' || echo "0")

    # Calculate error rate from socket errors
    local socket_errors=0
    if grep -q "Socket errors:" "${result_file}" 2>/dev/null; then
        local connect_err read_err write_err timeout_err
        connect_err=$(grep "Socket errors:" "${result_file}" | sed 's/.*connect \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
        read_err=$(grep "Socket errors:" "${result_file}" | sed 's/.*read \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
        write_err=$(grep "Socket errors:" "${result_file}" | sed 's/.*write \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
        timeout_err=$(grep "Socket errors:" "${result_file}" | sed 's/.*timeout \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
        socket_errors=$((connect_err + read_err + write_err + timeout_err))
    fi

    if [[ "${total_requests}" -gt 0 ]]; then
        error_rate=$(echo "scale=4; ${socket_errors} / ${total_requests}" | bc 2>/dev/null || echo "0")
    else
        error_rate="0"
    fi

    # Generate meta.json with keys matching requirements
    cat > "${meta_file}" << EOF
{
  "scenario": {
    "name": "${SCENARIO_NAME:-${script_name}}",
    "storage_mode": "${STORAGE_MODE:-unknown}",
    "cache_mode": "${CACHE_MODE:-unknown}",
    "data_scale": "${DATA_SCALE:-1e4}",
    "payload": "${PAYLOAD:-medium}",
    "rps_profile": "${RPS_PROFILE:-steady}",
    "hit_rate": ${HIT_RATE:-50},
    "cache_strategy": "${CACHE_STRATEGY:-read-through}",
    "fail_injection": ${FAIL_RATE:-0},
    "retry": ${RETRY:-false},
    "endpoint": "${ENDPOINT:-unknown}"
  },
  "execution": {
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "api_url": "${API_URL}",
    "threads": ${THREADS},
    "connections": ${CONNECTIONS},
    "duration": "${DURATION}",
    "workers": ${WORKERS:-0},
    "pool_sizes": ${POOL_SIZES:-0},
    "profile_enabled": ${PROFILE_MODE}
  },
  "results": {
    "rps": ${rps:-0},
    "total_requests": ${total_requests:-0},
    "avg_latency": "${avg_latency:-0}",
    "p50": "${p50:-0}",
    "p95": "${p95:-0}",
    "p99": "${p99:-0}",
    "error_rate": ${error_rate:-0},
    "socket_errors": ${socket_errors:-0}
  }
}
EOF

    echo -e "${GREEN}meta.json generated${NC}"
}

# =============================================================================
# Profiling Functions
# =============================================================================

check_profiling_tools() {
    if [[ "${PROFILE_MODE}" != "true" ]]; then
        return 0
    fi

    local has_tools=false

    if [[ "$(uname)" == "Linux" ]]; then
        if command -v perf &> /dev/null; then
            # Check if we can run perf (may need permissions)
            if perf list &> /dev/null || sudo -n perf list &> /dev/null 2>&1; then
                has_tools=true
            else
                echo -e "${YELLOW}Warning: perf is installed but may require sudo. Profiling may fail.${NC}"
                has_tools=true
            fi
        else
            echo -e "${YELLOW}Warning: perf not found. Install with: apt-get install linux-tools-common${NC}"
        fi
    elif [[ "$(uname)" == "Darwin" ]]; then
        if command -v sample &> /dev/null; then
            has_tools=true
        else
            echo -e "${YELLOW}Warning: sample command not found. CPU profiling unavailable on macOS.${NC}"
        fi
    fi

    # Check for flamegraph tools
    if ! command -v inferno-collapse-perf &> /dev/null && \
       [[ ! -f "/usr/local/share/FlameGraph/stackcollapse-perf.pl" ]] && \
       [[ ! -f "${FLAMEGRAPH_DIR:-/usr/local/share/FlameGraph}/stackcollapse-perf.pl" ]]; then
        echo -e "${YELLOW}Warning: FlameGraph tools not found. Install inferno: cargo install inferno${NC}"
    fi

    if [[ "${has_tools}" != "true" ]]; then
        echo -e "${YELLOW}Profiling tools not available. --profile will be ignored.${NC}"
        PROFILE_MODE=false
    fi
}

start_profiling() {
    if [[ "${PROFILE_MODE}" != "true" ]]; then
        return 0
    fi

    # Verify profiling tools are available (called here after function definition)
    check_profiling_tools
    if [[ "${PROFILE_MODE}" != "true" ]]; then
        return 0
    fi

    echo "Starting CPU profiling..."

    # Find API server PID
    local api_pid
    api_pid=$(pgrep -f "task-management-benchmark-api" 2>/dev/null | head -1 || true)

    if [[ -z "${api_pid}" ]]; then
        api_pid=$(pgrep -f "target/release/task" 2>/dev/null | head -1 || true)
    fi

    if [[ -z "${api_pid}" ]]; then
        echo -e "${YELLOW}Warning: Could not find API server process. Profiling skipped.${NC}"
        PROFILE_MODE=false
        return 0
    fi

    export PROFILE_PID="${api_pid}"
    PERF_DATA_FILE="${RESULTS_DIR}/perf.data"

    if [[ "$(uname)" == "Linux" ]]; then
        # Try without sudo first, fallback to sudo
        if perf record -F 99 -p "${api_pid}" -g -o "${PERF_DATA_FILE}" -- sleep 0 2>/dev/null; then
            rm -f "${PERF_DATA_FILE}" 2>/dev/null
            perf record -F 99 -p "${api_pid}" -g -o "${PERF_DATA_FILE}" &
            export PERF_RECORD_PID=$!
        elif sudo -n true 2>/dev/null; then
            sudo perf record -F 99 -p "${api_pid}" -g -o "${PERF_DATA_FILE}" &
            export PERF_RECORD_PID=$!
            export PERF_NEEDS_SUDO=true
        else
            echo -e "${YELLOW}Warning: Cannot run perf (permission denied). Skipping profiling.${NC}"
            PROFILE_MODE=false
            return 0
        fi
        echo "  perf recording started (PID: ${api_pid})"
    elif [[ "$(uname)" == "Darwin" ]]; then
        # macOS: use sample command
        local duration_secs="${DURATION%s}"
        sample "${api_pid}" "${duration_secs}" -f "${RESULTS_DIR}/sample.txt" &
        export SAMPLE_PID=$!
        echo "  sample recording started (PID: ${api_pid}, duration: ${duration_secs}s)"
    fi
}

stop_profiling() {
    if [[ "${PROFILE_MODE}" != "true" ]]; then
        return 0
    fi

    echo "Stopping profiling..."

    if [[ -n "${PERF_RECORD_PID:-}" ]]; then
        if [[ "${PERF_NEEDS_SUDO:-}" == "true" ]]; then
            sudo kill -INT "${PERF_RECORD_PID}" 2>/dev/null || true
        else
            kill -INT "${PERF_RECORD_PID}" 2>/dev/null || true
        fi
        wait "${PERF_RECORD_PID}" 2>/dev/null || true
    fi

    if [[ -n "${SAMPLE_PID:-}" ]]; then
        wait "${SAMPLE_PID}" 2>/dev/null || true
    fi
}

generate_flamegraph() {
    if [[ "${PROFILE_MODE}" != "true" ]]; then
        return 0
    fi

    local flamegraph_svg="${RESULTS_DIR}/flamegraph.svg"
    local collapsed_file="${RESULTS_DIR}/stacks.folded"

    echo "Generating flamegraph..."

    if [[ "$(uname)" == "Linux" ]]; then
        local perf_data="${RESULTS_DIR}/perf.data"

        if [[ ! -f "${perf_data}" ]]; then
            echo -e "${YELLOW}Warning: No perf.data found. Skipping flamegraph generation.${NC}"
            return 0
        fi

        # Generate collapsed stacks
        local perf_script_cmd="perf script -i ${perf_data}"
        if [[ "${PERF_NEEDS_SUDO:-}" == "true" ]]; then
            perf_script_cmd="sudo perf script -i ${perf_data}"
        fi

        if command -v inferno-collapse-perf &> /dev/null; then
            eval "${perf_script_cmd}" 2>/dev/null | \
                inferno-collapse-perf > "${collapsed_file}" 2>/dev/null || true

            if [[ -s "${collapsed_file}" ]]; then
                inferno-flamegraph < "${collapsed_file}" > "${flamegraph_svg}" 2>/dev/null || true
            fi
        elif [[ -f "${FLAMEGRAPH_DIR:-/usr/local/share/FlameGraph}/stackcollapse-perf.pl" ]]; then
            local fg_dir="${FLAMEGRAPH_DIR:-/usr/local/share/FlameGraph}"
            eval "${perf_script_cmd}" 2>/dev/null | \
                "${fg_dir}/stackcollapse-perf.pl" > "${collapsed_file}" 2>/dev/null || true

            if [[ -s "${collapsed_file}" ]]; then
                "${fg_dir}/flamegraph.pl" < "${collapsed_file}" > "${flamegraph_svg}" 2>/dev/null || true
            fi
        else
            echo -e "${YELLOW}Warning: FlameGraph tools not found. Skipping flamegraph generation.${NC}"
            return 0
        fi

    elif [[ "$(uname)" == "Darwin" ]]; then
        local sample_file="${RESULTS_DIR}/sample.txt"

        if [[ ! -f "${sample_file}" ]]; then
            echo -e "${YELLOW}Warning: No sample.txt found. Skipping flamegraph generation.${NC}"
            return 0
        fi

        # macOS: Convert sample output to flamegraph
        if command -v inferno-collapse-sample &> /dev/null; then
            inferno-collapse-sample < "${sample_file}" > "${collapsed_file}" 2>/dev/null || true

            if [[ -s "${collapsed_file}" ]]; then
                inferno-flamegraph < "${collapsed_file}" > "${flamegraph_svg}" 2>/dev/null || true
            fi
        elif [[ -f "${FLAMEGRAPH_DIR:-/usr/local/share/FlameGraph}/stackcollapse-sample.awk" ]]; then
            local fg_dir="${FLAMEGRAPH_DIR:-/usr/local/share/FlameGraph}"
            awk -f "${fg_dir}/stackcollapse-sample.awk" "${sample_file}" > "${collapsed_file}" 2>/dev/null || true

            if [[ -s "${collapsed_file}" ]]; then
                "${fg_dir}/flamegraph.pl" < "${collapsed_file}" > "${flamegraph_svg}" 2>/dev/null || true
            fi
        else
            echo -e "${YELLOW}Warning: FlameGraph tools not found for macOS. Install inferno: cargo install inferno${NC}"
            return 0
        fi
    fi

    if [[ -f "${flamegraph_svg}" ]] && [[ -s "${flamegraph_svg}" ]]; then
        echo -e "${GREEN}flamegraph.svg generated${NC}"
    else
        echo -e "${YELLOW}Warning: flamegraph.svg generation failed or empty${NC}"
    fi
}

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

    # Create script-specific result directory when running multiple scripts
    # (including --scenario mode without --specific)
    local script_results_dir="${RESULTS_DIR}"
    if [[ -z "${SPECIFIC_SCRIPT}" ]]; then
        # Multiple scripts mode: create subdirectory for each script
        script_results_dir="${RESULTS_DIR}/${script_name}"
        mkdir -p "${script_results_dir}"
    fi

    local result_file="${script_results_dir}/wrk.txt"

    # Start profiling if enabled
    start_profiling

    # Run wrk and capture output (with --latency for percentile stats)
    cd "${SCRIPT_DIR}"
    if wrk -t"${THREADS}" -c"${CONNECTIONS}" -d"${DURATION}" \
        --latency \
        --script="scripts/${script_name}.lua" \
        "${API_URL}" 2>&1 | tee "${result_file}"; then

        # Stop profiling
        stop_profiling

        # Generate flamegraph if profiling was enabled (save to script-specific dir)
        local orig_results_dir="${RESULTS_DIR}"
        RESULTS_DIR="${script_results_dir}"
        generate_flamegraph
        RESULTS_DIR="${orig_results_dir}"

        # Extract key metrics for summary
        local reqs_sec=$(grep "Requests/sec:" "${result_file}" | awk '{print $2}')
        local avg_latency=$(grep "Latency" "${result_file}" | head -1 | awk '{print $2}')

        # Extract latency percentiles (P50, P75, P90, P99)
        local p50=$(grep "50%" "${result_file}" | awk '{print $2}')
        local p75=$(grep "75%" "${result_file}" | awk '{print $2}')
        local p90=$(grep "90%" "${result_file}" | awk '{print $2}')
        local p99=$(grep "99%" "${result_file}" | awk '{print $2}')

        echo "" >> "${SUMMARY_FILE}"
        echo "${script_name}:" >> "${SUMMARY_FILE}"
        echo "  Requests/sec: ${reqs_sec:-N/A}" >> "${SUMMARY_FILE}"
        echo "  Avg Latency:  ${avg_latency:-N/A}" >> "${SUMMARY_FILE}"
        echo "  P50: ${p50:-N/A}" >> "${SUMMARY_FILE}"
        echo "  P75: ${p75:-N/A}" >> "${SUMMARY_FILE}"
        echo "  P90: ${p90:-N/A}" >> "${SUMMARY_FILE}"
        echo "  P99: ${p99:-N/A}" >> "${SUMMARY_FILE}"

        # Generate meta.json in script-specific directory
        local orig_results_dir="${RESULTS_DIR}"
        RESULTS_DIR="${script_results_dir}"
        generate_meta_json "${result_file}" "${script_name}"
        RESULTS_DIR="${orig_results_dir}"

        echo -e "${GREEN}Completed${NC}"
    else
        # Stop profiling even on failure
        stop_profiling

        echo -e "${RED}Failed${NC}"
        echo "${script_name}: FAILED" >> "${SUMMARY_FILE}"
    fi
}

# Get list of scripts to run
if [[ -n "${SPECIFIC_SCRIPT}" ]]; then
    SCRIPTS=("${SPECIFIC_SCRIPT}")
else
    SCRIPTS=(
        "recursive"
        "ordered"
        "traversable"
        "alternative"
        "async_pipeline"
        "bifunctor"
        "applicative"
        "optics"
        "misc"
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

# Generate bottleneck analysis
echo ""
echo "=============================================="
echo "  Bottleneck Analysis"
echo "=============================================="
echo "" >> "${SUMMARY_FILE}"
echo "--- Bottleneck Analysis ---" >> "${SUMMARY_FILE}"

# Find slowest endpoint (lowest Requests/sec)
slowest_endpoint=""
slowest_rps=999999999
highest_p99=""
highest_p99_endpoint=""

# Search for result files in both top-level and subdirectories
# Use find to locate all wrk.txt or *.txt files (excluding summary.txt)
result_files=()
while IFS= read -r -d '' file; do
    result_files+=("$file")
done < <(find "${RESULTS_DIR}" -name "wrk.txt" -type f -print0 2>/dev/null)

# Also check for legacy top-level .txt files (excluding summary.txt)
for txt_file in "${RESULTS_DIR}"/*.txt; do
    if [[ -f "${txt_file}" ]] && [[ "$(basename "${txt_file}")" != "summary.txt" ]]; then
        result_files+=("${txt_file}")
    fi
done

for result_file in "${result_files[@]}"; do
    if [ -f "$result_file" ]; then
        # Determine endpoint name from path
        dir_name=$(dirname "$result_file")
        if [[ "$dir_name" == "${RESULTS_DIR}" ]]; then
            # Legacy top-level file: endpoint name is filename without extension
            endpoint=$(basename "$result_file" .txt)
        else
            # Subdirectory: endpoint name is the directory name
            endpoint=$(basename "$dir_name")
        fi
        rps=$(grep "Requests/sec:" "$result_file" 2>/dev/null | awk '{print $2}' | sed 's/[^0-9.]//g')
        p99=$(grep "99%" "$result_file" 2>/dev/null | awk '{print $2}')

        if [ -n "$rps" ]; then
            # Compare as integers (multiply by 100 to handle decimals)
            rps_int=$(echo "$rps" | awk '{printf "%.0f", $1 * 100}')
            slowest_int=$(echo "$slowest_rps" | awk '{printf "%.0f", $1 * 100}')

            if [ "$rps_int" -lt "$slowest_int" ]; then
                slowest_rps="$rps"
                slowest_endpoint="$endpoint"
            fi
        fi

        # Track highest P99 latency
        if [ -n "$p99" ]; then
            # Extract numeric value (remove units like 'ms', 's')
            p99_num=$(echo "$p99" | sed 's/[^0-9.]//g')
            p99_unit=$(echo "$p99" | sed 's/[0-9.]//g')

            # Convert to microseconds for comparison
            case "$p99_unit" in
                us) p99_us="$p99_num" ;;
                ms) p99_us=$(echo "$p99_num" | awk '{printf "%.0f", $1 * 1000}') ;;
                s)  p99_us=$(echo "$p99_num" | awk '{printf "%.0f", $1 * 1000000}') ;;
                *)  p99_us="$p99_num" ;;
            esac

            if [ -z "$highest_p99" ] || [ "$p99_us" -gt "$highest_p99" ]; then
                highest_p99="$p99_us"
                highest_p99_endpoint="$endpoint ($p99)"
            fi
        fi
    fi
done

if [ -n "$slowest_endpoint" ]; then
    echo -e "${YELLOW}Slowest endpoint: ${slowest_endpoint} (${slowest_rps} req/s)${NC}"
    echo "Slowest endpoint: ${slowest_endpoint} (${slowest_rps} req/s)" >> "${SUMMARY_FILE}"
fi

if [ -n "$highest_p99_endpoint" ]; then
    echo -e "${YELLOW}Highest P99 latency: ${highest_p99_endpoint}${NC}"
    echo "Highest P99 latency: ${highest_p99_endpoint}" >> "${SUMMARY_FILE}"
fi

echo ""
