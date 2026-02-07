#!/bin/bash
# benches/api/benchmarks/run_benchmark.sh
#
# Run wrk/wrk2 benchmarks for API endpoints
#
# =============================================================================
# RESPONSIBILITY SEPARATION
# =============================================================================
#
# This script is responsible ONLY for:
#   - Executing wrk/wrk2 load generation
#   - Collecting benchmark results
#   - Generating meta.json with execution metadata
#
# API startup and environment variable injection are delegated to:
#   - xtask (cargo xtask bench-api): Parses scenario YAML, generates env vars,
#     starts/stops API server, and invokes this script
#   - Manual startup: User starts API with appropriate env vars before running
#
# Environment variables are expected to be:
#   - Inherited from xtask (when using cargo xtask bench-api)
#   - Set by the user (when running this script directly)
#
# =============================================================================
# Usage:
#   ./run_benchmark.sh --scenario <yaml>                # Run with scenario configuration (REQUIRED)
#   ./run_benchmark.sh --scenario <yaml> --quick        # Quick test (5s duration)
#   ./run_benchmark.sh --scenario <yaml> --profile      # Run with perf profiling
#   ./run_benchmark.sh --scenario <yaml> --quick --profile  # Combined options
#
# Recommended usage (via xtask for full environment integration):
#   cargo xtask bench-api --scenario <yaml>
#
# IMPORTANT: --scenario is REQUIRED. Use one of the scenarios in benches/api/benchmarks/scenarios/
#
# Environment Variables (set via xtask, scenario YAML parsing, or directly):
#   API_URL          - API server URL (default: http://localhost:3002)
#   STORAGE_MODE     - in_memory | postgres (REQUIRED)
#   CACHE_MODE       - in_memory | redis | none (REQUIRED)
#   DATA_SCALE       - 1e2 | 1e4 | 1e6 (maps from small/medium/large) (REQUIRED)
#   HIT_RATE         - 0-100 (default: 50)
#   CACHE_STRATEGY   - read-through | write-through | write-behind (default: read-through)
#   RPS_PROFILE      - steady | ramp_up_down | burst | step_up (constant is alias for steady)
#   THREADS          - wrk threads
#   CONNECTIONS      - wrk connections
#   DURATION         - wrk duration
#   POOL_SIZES       - DB+Redis pool size (combined)
#   DATABASE_POOL_SIZE - Database pool size (default: 16)
#   REDIS_POOL_SIZE  - Redis pool size (default: 8)
#   WORKER_THREADS   - tokio worker threads (default: 4)
#   FAIL_RATE        - 0.0-1.0 (default: 0)
#   RETRY            - true | false (default: false)
#   PROFILE          - true | false (default: false)
#   ENDPOINT         - target endpoint
#   PAYLOAD          - small | medium | large (default: medium)
#                      (also accepts minimal | standard | complex | heavy, mapped to small/medium/large)
#   RPS_TOLERANCE_MODE - strict | warn (default: strict)
#                        strict: Fail benchmark if actual RPS deviates beyond tolerance
#                        warn: Log warning but continue execution

set -euo pipefail

# Configuration
API_URL="${API_URL:-http://localhost:3002}"
THREADS="${THREADS:-2}"
CONNECTIONS="${CONNECTIONS:-10}"
DURATION="${DURATION:-30s}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
RESULTS_DIR="${SCRIPT_DIR}/results/${TIMESTAMP}"

# Source scenario environment utilities
# shellcheck source=scripts/scenario_env.sh
source "${SCRIPT_DIR}/scripts/scenario_env.sh"

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

# Support PROFILE environment variable for CI compatibility
# This allows enabling profiling via env var without --profile flag
if [[ "${PROFILE:-}" == "true" && "${PROFILE_MODE}" == "false" ]]; then
    PROFILE_MODE=true
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# =============================================================================
# Scenario Required Check
# =============================================================================

list_available_scenarios() {
    local scenarios_dir="${SCRIPT_DIR}/scenarios"
    echo -e "${CYAN}Available scenarios:${NC}"
    echo ""
    for scenario in "${scenarios_dir}"/*.yaml; do
        if [[ -f "${scenario}" ]]; then
            local name
            name=$(basename "${scenario}" .yaml)
            echo "  - ${name}"
        fi
    done
    echo ""
    echo "Usage: $0 --scenario <scenario_name_or_path> [--quick] [--profile]"
    echo ""
    echo "Examples:"
    echo "  $0 --scenario tasks_eff"
    echo "  $0 --scenario scenarios/tasks_eff.yaml --quick"
    echo "  $0 --scenario postgres_redis_read_heavy_warm --profile"
}

if [[ -z "${SCENARIO_FILE}" ]]; then
    echo -e "${RED}Error: --scenario option is REQUIRED${NC}"
    echo ""
    list_available_scenarios
    exit 1
fi

# Resolve scenario file path
if [[ ! -f "${SCENARIO_FILE}" ]]; then
    # Try to find in scenarios directory
    if [[ -f "${SCRIPT_DIR}/scenarios/${SCENARIO_FILE}" ]]; then
        SCENARIO_FILE="${SCRIPT_DIR}/scenarios/${SCENARIO_FILE}"
    elif [[ -f "${SCRIPT_DIR}/scenarios/${SCENARIO_FILE}.yaml" ]]; then
        SCENARIO_FILE="${SCRIPT_DIR}/scenarios/${SCENARIO_FILE}.yaml"
    else
        echo -e "${RED}Error: Scenario file not found: ${SCENARIO_FILE}${NC}"
        echo ""
        list_available_scenarios
        exit 1
    fi
fi

# =============================================================================
# Endpoint to Lua Script Mapping
# =============================================================================
#
# Maps API endpoints to their corresponding Lua benchmark scripts.
# This enables scenario-driven script selection based on metadata.endpoint.
#
# Mapping:
#   "POST /tasks-eff"               -> tasks_eff
#   "POST /tasks/bulk"              -> tasks_bulk
#   "POST /tasks/search"            -> tasks_search
#   "PUT /tasks/{id}"               -> tasks_update
#   "GET /projects/{id}/progress"   -> projects_progress
#   Generic endpoints               -> legacy scripts (recursive, ordered, etc.)
# =============================================================================

resolve_script_from_endpoint() {
    local endpoint="$1"

    # Normalize endpoint: remove method prefix if present
    local path
    path=$(echo "${endpoint}" | sed 's/^[A-Z]* *//' | tr -d ' ')

    case "${path}" in
        "/tasks-eff")               echo "tasks_eff" ;;
        "/tasks/bulk")              echo "tasks_bulk" ;;
        "/tasks/search")            echo "tasks_search" ;;
        "/tasks/{id}"|"/tasks/*")   echo "tasks_update" ;;
        "/projects/{id}/progress"|"/projects/*/progress")  echo "projects_progress" ;;
        "/tasks")                   echo "recursive" ;;
        "/health")                  echo "health" ;;
        *)                          echo "" ;;  # Unknown endpoint
    esac
}

# Resolve scripts from scenario configuration
resolve_scripts_from_scenario() {
    local scenario_file="$1"
    local scripts=()

    # Check if yq is available
    if ! command -v yq &> /dev/null; then
        echo -e "${YELLOW}Warning: yq not available, using grep fallback for script resolution${NC}" >&2
        # Fallback: try to extract endpoint from metadata using grep
        local endpoint
        endpoint=$(grep -E '^\s*endpoint:' "${scenario_file}" 2>/dev/null | head -1 | sed 's/.*endpoint: *"\?\([^"]*\)"\?/\1/' | tr -d '"')
        if [[ -n "${endpoint}" ]]; then
            local script
            script=$(resolve_script_from_endpoint "${endpoint}")
            if [[ -n "${script}" ]]; then
                echo "${script}"
                return 0
            fi
        fi
        if [[ "${endpoint}" == "mixed" && -n "${MIXED_SCRIPT:-}" ]]; then
            if [[ -f "${SCRIPT_DIR}/scripts/${MIXED_SCRIPT}.lua" ]]; then
                echo "${MIXED_SCRIPT}"
                return 0
            else
                echo -e "${YELLOW}Warning: MIXED_SCRIPT ${MIXED_SCRIPT}.lua not found, falling back to legacy scripts${NC}" >&2
            fi
        fi
        # Fallback to legacy scripts
        echo "recursive ordered traversable alternative async_pipeline bifunctor applicative optics misc"
        return 0
    fi

    # Try metadata.endpoint first (single endpoint scenario)
    local metadata_endpoint
    metadata_endpoint=$(yq '.metadata.endpoint // null' "${scenario_file}" | tr -d '"')

    # If metadata.endpoint is "mixed", always use legacy scripts for full coverage
    if [[ "${metadata_endpoint}" == "mixed" ]]; then
        if [[ -n "${MIXED_SCRIPT:-}" ]]; then
            if [[ -f "${SCRIPT_DIR}/scripts/${MIXED_SCRIPT}.lua" ]]; then
                echo "${MIXED_SCRIPT}"
                return 0
            else
                echo -e "${YELLOW}Warning: MIXED_SCRIPT ${MIXED_SCRIPT}.lua not found, falling back to legacy scripts${NC}" >&2
            fi
        fi
        echo "recursive ordered traversable alternative async_pipeline bifunctor applicative optics misc"
        return 0
    fi

    if [[ "${metadata_endpoint}" != "null" && -n "${metadata_endpoint}" ]]; then
        local script
        script=$(resolve_script_from_endpoint "${metadata_endpoint}")
        if [[ -n "${script}" ]]; then
            # Verify the script exists
            if [[ -f "${SCRIPT_DIR}/scripts/${script}.lua" ]]; then
                echo "${script}"
                return 0
            else
                echo -e "${YELLOW}Warning: Script ${script}.lua not found for endpoint ${metadata_endpoint}${NC}" >&2
            fi
        fi
    fi

    # Try endpoints array (multi-endpoint scenario)
    local endpoints_count
    endpoints_count=$(yq '.endpoints | length // 0' "${scenario_file}")

    if [[ "${endpoints_count}" -gt 0 ]]; then
        for i in $(seq 0 $((endpoints_count - 1))); do
            local ep
            ep=$(yq ".endpoints[${i}]" "${scenario_file}" | tr -d '"')
            local script
            script=$(resolve_script_from_endpoint "${ep}")
            if [[ -n "${script}" && -f "${SCRIPT_DIR}/scripts/${script}.lua" ]]; then
                scripts+=("${script}")
            fi
        done

        if [[ ${#scripts[@]} -gt 0 ]]; then
            # Remove duplicates and return
            echo "${scripts[@]}" | tr ' ' '\n' | sort -u | tr '\n' ' '
            return 0
        fi
    fi

    # Fallback: use legacy scripts for general scenarios
    echo "recursive ordered traversable alternative async_pipeline bifunctor applicative optics misc"
}

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
#   rps_profile (steady/ramp_up_down/burst/step_up) -> RPS_PROFILE, LOAD_PROFILE (constant is alias for steady)
#   threads                           -> THREADS
#   connections                       -> CONNECTIONS
#   duration_seconds                  -> DURATION
#   concurrency.worker_threads        -> WORKER_THREADS
#   concurrency.database_pool_size + redis_pool_size -> POOL_SIZES
#   cache_metrics.expected_hit_rate or metadata.hit_rate -> HIT_RATE (0/50/90)
#   metadata.cache_strategy           -> CACHE_STRATEGY
#   error_config.inject_error_rate or metadata.fail_injection -> FAIL_RATE
#   error_config.max_retries > 0 or metadata.retry -> RETRY
#   profiling.enable_perf or metadata.profile -> PROFILE
#   endpoints[0] or metadata.endpoint -> ENDPOINT
#   MIXED_SCRIPT                   -> mixed endpoint override (single script)
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

    # Storage mode (preserve existing environment variable if set)
    local storage_mode
    storage_mode=$(yq '.storage_mode // "in_memory"' "${scenario_file}" | tr -d '"')
    export STORAGE_MODE="${STORAGE_MODE:-${storage_mode}}"

    # Cache mode (preserve existing environment variable if set)
    local cache_mode
    cache_mode=$(yq '.cache_mode // "in_memory"' "${scenario_file}" | tr -d '"')
    export CACHE_MODE="${CACHE_MODE:-${cache_mode}}"

    # Data scale: small -> 1e2, medium -> 1e4, large -> 1e6
    # Use environment variable if set, otherwise read from YAML
    local data_scale_raw
    local data_scale_source="yaml"

    if [[ -n "${DATA_SCALE:-}" ]]; then
        data_scale_raw="${DATA_SCALE}"
        data_scale_source="env"
    else
        data_scale_raw=$(yq '.data_scale // "medium"' "${scenario_file}" | tr -d '"')
    fi

    # Map human-readable format to numeric format
    case "${data_scale_raw}" in
        "small"|"1e2")  export DATA_SCALE="1e2" ;;
        "medium"|"1e4") export DATA_SCALE="1e4" ;;
        "large"|"1e6")  export DATA_SCALE="1e6" ;;
        *)
            if [[ "${data_scale_source}" == "env" ]]; then
                echo -e "${RED}Error: Invalid DATA_SCALE '${data_scale_raw}' from environment. Must be: small|medium|large|1e2|1e4|1e6${NC}" >&2
                return 1
            else
                echo -e "${YELLOW}Warning: Unknown data_scale '${data_scale_raw}' in scenario, defaulting to medium (1e4)${NC}" >&2
                export DATA_SCALE="1e4"
            fi
            ;;
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
    # Only set PAYLOAD if not already set from environment (allows CI variants to override)
    if [[ -z "${PAYLOAD:-}" ]]; then
        export PAYLOAD="${payload}"
    fi

    # RPS profile: Map scenario values to load_profile.lua profile names
    # Supported profiles: steady, ramp_up_down, burst, step_up
    local rps_profile
    rps_profile=$(yq '.rps_profile // "steady"' "${scenario_file}" | tr -d '"')
    case "${rps_profile}" in
        "steady"|"constant") export RPS_PROFILE="steady" ;;
        "ramp_up_down")      export RPS_PROFILE="ramp_up_down" ;;
        "burst")             export RPS_PROFILE="burst" ;;
        "step_up")           export RPS_PROFILE="step_up" ;;
        *)
            echo -e "${YELLOW}WARNING: Unknown rps_profile '${rps_profile}', defaulting to steady${NC}"
            export RPS_PROFILE="steady"
            ;;
    esac
    # LOAD_PROFILE is used by Lua scripts (load_profile.lua)
    export LOAD_PROFILE="${RPS_PROFILE}"

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

    # WRK_THREADS: Number of wrk threads for Lua script thread-local state
    # Used by tasks_update.lua for ID range partitioning
    if [[ -z "${WRK_THREADS:-}" ]]; then
        export WRK_THREADS="${THREADS}"
    fi

    # ==========================================================================
    # Concurrency Settings (WORKER_THREADS, POOL_SIZES)
    # ==========================================================================
    # Priority: Environment > .concurrency.* > .worker_config.* > .pool_sizes.*
    # Existing environment variables are preserved (not overwritten by scenario)

    # Workers: prefer concurrency.worker_threads, fallback to worker_config.worker_threads
    # Only set if WORKER_THREADS is not already set
    if [[ -z "${WORKER_THREADS:-}" ]]; then
        local worker_threads
        worker_threads=$(yq '.concurrency.worker_threads // null' "${scenario_file}")
        if [[ "${worker_threads}" == "null" ]]; then
            worker_threads=$(yq '.worker_config.worker_threads // null' "${scenario_file}")
        fi
        if [[ "${worker_threads}" != "null" ]]; then
            export WORKER_THREADS="${worker_threads}"
        fi
    fi

    # Pool sizes: prefer concurrency.*, fallback to pool_sizes.*
    # Only set if not already set from environment
    local database_pool_size redis_pool_size

    # Try concurrency.* first
    database_pool_size=$(yq '.concurrency.database_pool_size // null' "${scenario_file}")
    redis_pool_size=$(yq '.concurrency.redis_pool_size // null' "${scenario_file}")

    # Fallback to pool_sizes.* if concurrency.* not set
    if [[ "${database_pool_size}" == "null" ]]; then
        database_pool_size=$(yq '.pool_sizes.database_pool_size // 0' "${scenario_file}")
    fi
    if [[ "${redis_pool_size}" == "null" ]]; then
        redis_pool_size=$(yq '.pool_sizes.redis_pool_size // 0' "${scenario_file}")
    fi

    # Set environment variables if values are present, preserving existing values
    if [[ "${database_pool_size}" != "null" && "${database_pool_size}" != "0" ]] || \
       [[ "${redis_pool_size}" != "null" && "${redis_pool_size}" != "0" ]]; then
        database_pool_size=${database_pool_size:-0}
        redis_pool_size=${redis_pool_size:-0}
        local pool_sizes=$((database_pool_size + redis_pool_size))
        export POOL_SIZES="${POOL_SIZES:-${pool_sizes}}"
        export DATABASE_POOL_SIZE="${DATABASE_POOL_SIZE:-${database_pool_size}}"
        export REDIS_POOL_SIZE="${REDIS_POOL_SIZE:-${redis_pool_size}}"
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
    # Only set HIT_RATE if not already set from environment (allows CI variants to override)
    if [[ -z "${HIT_RATE:-}" ]]; then
        export HIT_RATE="${hit_rate}"
    fi

    # Cache strategy (preserve existing environment variable if set)
    local cache_strategy
    cache_strategy=$(yq '.metadata.cache_strategy // "read-through"' "${scenario_file}" | tr -d '"')
    export CACHE_STRATEGY="${CACHE_STRATEGY:-${cache_strategy}}"

    # ==========================================================================
    # Cache Metrics Configuration
    # ==========================================================================
    # Export cache_metrics section values as environment variables for
    # API server and warmup logic.

    # CACHE_METRICS_ENABLED: Whether cache metrics are enabled (1 or 0)
    local cache_metrics_enabled_raw
    cache_metrics_enabled_raw=$(yq '.cache_metrics.enabled // false' "${scenario_file}" | tr -d '"')
    # Normalize to 1/0 for Lua compatibility
    if [[ "${cache_metrics_enabled_raw}" == "true" ]]; then
        export CACHE_METRICS_ENABLED="1"
    else
        export CACHE_METRICS_ENABLED="0"
    fi

    # CACHE_WARMUP_REQUESTS: Number of warmup requests to send before measurement
    local cache_warmup_requests
    cache_warmup_requests=$(yq '.cache_metrics.warmup_requests // 0' "${scenario_file}")
    export CACHE_WARMUP_REQUESTS="${cache_warmup_requests}"

    # EXPECTED_CACHE_HIT_RATE: Expected cache hit rate threshold (0.0-1.0)
    local expected_cache_hit_rate
    expected_cache_hit_rate=$(yq '.cache_metrics.expected_hit_rate // ""' "${scenario_file}" | tr -d '"')
    if [[ -n "${expected_cache_hit_rate}" ]]; then
        export EXPECTED_CACHE_HIT_RATE="${expected_cache_hit_rate}"
    fi

    # CACHE_METRICS_PER_ENDPOINT: Track cache hit rate per endpoint (1 or 0)
    local cache_metrics_per_endpoint_raw
    cache_metrics_per_endpoint_raw=$(yq '.cache_metrics.per_endpoint // false' "${scenario_file}" | tr -d '"')
    # Normalize to 1/0 for Lua compatibility
    if [[ "${cache_metrics_per_endpoint_raw}" == "true" ]]; then
        export CACHE_METRICS_PER_ENDPOINT="1"
    else
        export CACHE_METRICS_PER_ENDPOINT="0"
    fi

    # CACHE_METRICS_TRACK_LATENCY: Track cache latency distribution (1 or 0)
    local cache_metrics_track_latency_raw
    cache_metrics_track_latency_raw=$(yq '.cache_metrics.track_latency // false' "${scenario_file}" | tr -d '"')
    # Normalize to 1/0 for Lua compatibility
    if [[ "${cache_metrics_track_latency_raw}" == "true" ]]; then
        export CACHE_METRICS_TRACK_LATENCY="1"
    else
        export CACHE_METRICS_TRACK_LATENCY="0"
    fi

    # ==========================================================================
    # Error Configuration (FAIL_RATE, RETRY)
    # ==========================================================================

    # Fail injection rate: prefer metadata.fail_injection, fallback to error_config.inject_error_rate
    # Only set FAIL_RATE if not already set from environment (allows CI variants to override)
    if [[ -z "${FAIL_RATE:-}" ]]; then
        local fail_injection
        fail_injection=$(yq '.metadata.fail_injection // null' "${scenario_file}")
        if [[ "${fail_injection}" == "null" ]]; then
            fail_injection=$(yq '.error_config.inject_error_rate // 0' "${scenario_file}")
        fi
        export FAIL_RATE="${fail_injection}"
    fi

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
    # Tasks Update Configuration (ID_POOL_SIZE, RETRY_COUNT)
    # ==========================================================================

    load_numeric_param() {
        local env_var="$1"
        local yaml_path1="$2"
        local yaml_path2="$3"
        local default_value="$4"
        local param_name="$5"

        if [[ -n "${!env_var:-}" ]]; then
            return 0
        fi

        local value
        value=$(yq -r -e "${yaml_path1}" "${scenario_file}" 2>/dev/null) || value="null"
        if [[ "${value}" == "null" || -z "${value}" ]]; then
            value=$(yq -r "${yaml_path2}" "${scenario_file}" 2>/dev/null) || value="${default_value}"
        fi

        if ! [[ "${value}" =~ ^[0-9]+$ ]]; then
            echo -e "${YELLOW}WARNING: Invalid ${param_name} '${value}', using default ${default_value}${NC}" >&2
            value="${default_value}"
        fi

        export "${env_var}=${value}"
    }

    load_numeric_param "ID_POOL_SIZE" ".metadata.id_pool_size // null" ".id_pool_size // 10" "10" "id_pool_size"
    load_numeric_param "RETRY_COUNT" ".metadata.retry_count // null" ".error_config.max_retries // 0" "0" "retry_count"

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
    # Multi-Phase Load Profile Parameters
    # ==========================================================================
    # These parameters configure the phased benchmark execution.
    # See run_phased_benchmark() for how these are used.

    # MIN_RPS: Minimum RPS floor (default: 10)
    local min_rps_val
    min_rps_val=$(yq '.min_rps // 10' "${scenario_file}")
    export MIN_RPS="${min_rps_val}"

    # STEP_COUNT: Number of steps for step_up profile (default: 4)
    local step_count_val
    step_count_val=$(yq '.step_count // 4' "${scenario_file}")
    export STEP_COUNT="${step_count_val}"

    # RAMP_UP_SECONDS: Duration of ramp up phase (default: 10)
    local ramp_up_val
    ramp_up_val=$(yq '.ramp_up_seconds // 10' "${scenario_file}")
    export RAMP_UP_SECONDS="${ramp_up_val}"

    # RAMP_DOWN_SECONDS: Duration of ramp down phase (default: 10)
    local ramp_down_val
    ramp_down_val=$(yq '.ramp_down_seconds // 10' "${scenario_file}")
    export RAMP_DOWN_SECONDS="${ramp_down_val}"

    # BURST_INTERVAL_SECONDS: Interval between bursts (default: 20)
    local burst_interval_val
    burst_interval_val=$(yq '.burst_interval_seconds // 20' "${scenario_file}")
    export BURST_INTERVAL_SECONDS="${burst_interval_val}"

    # BURST_DURATION_SECONDS: Duration of each burst (default: 5)
    local burst_duration_val
    burst_duration_val=$(yq '.burst_duration_seconds // 5' "${scenario_file}")
    export BURST_DURATION_SECONDS="${burst_duration_val}"

    # BURST_MULTIPLIER: RPS multiplier during burst (default: 3)
    local burst_multiplier_val
    burst_multiplier_val=$(yq '.burst_multiplier // 3' "${scenario_file}")
    export BURST_MULTIPLIER="${burst_multiplier_val}"

    # Seed for reproducible data generation
    local seed
    seed=$(yq '.seed // null' "${scenario_file}")
    if [[ "${seed}" != "null" && -n "${seed}" ]]; then
        export SEED="${seed}"
    fi

    # ==========================================================================
    # Environment Section (Cache Configuration)
    # ==========================================================================
    # Export environment variables from the 'environment' section in the scenario YAML.
    # These variables are passed to the API server at startup for cache configuration.

    # CACHE_ENABLED from environment section (default: true)
    local cache_enabled
    cache_enabled=$(yq '.environment.CACHE_ENABLED // "true"' "${scenario_file}" | tr -d '"')
    export CACHE_ENABLED="${cache_enabled}"

    # CACHE_STRATEGY from environment section (takes precedence over metadata.cache_strategy)
    local env_cache_strategy
    env_cache_strategy=$(yq '.environment.CACHE_STRATEGY // null' "${scenario_file}" | tr -d '"')
    if [[ "${env_cache_strategy}" != "null" && -n "${env_cache_strategy}" ]]; then
        export CACHE_STRATEGY="${env_cache_strategy}"
    fi

    # CACHE_TTL_SECS from environment section (default: 60)
    local cache_ttl
    cache_ttl=$(yq '.environment.CACHE_TTL_SECS // "60"' "${scenario_file}" | tr -d '"')
    export CACHE_TTL_SECS="${cache_ttl}"

    # ==========================================================================
    # Summary Output
    # ==========================================================================

    echo "  Scenario: ${SCENARIO_NAME}"
    echo "  Storage: ${STORAGE_MODE}, Cache: ${CACHE_MODE}"
    echo "  Data scale: ${DATA_SCALE}, Payload: ${PAYLOAD}"
    echo "  RPS profile: ${RPS_PROFILE}, Hit rate: ${HIT_RATE}%"
    echo "  Cache strategy: ${CACHE_STRATEGY}"
    echo "  Cache enabled: ${CACHE_ENABLED}, TTL: ${CACHE_TTL_SECS}s"
    echo "  Fail rate: ${FAIL_RATE}, Retry: ${RETRY}"
    [[ -n "${ENDPOINT:-}" ]] && echo "  Endpoint: ${ENDPOINT}"
    [[ -n "${WORKER_THREADS:-}" ]] && echo "  Worker threads: ${WORKER_THREADS}"
    [[ -n "${POOL_SIZES:-}" ]] && echo "  Pool sizes: ${POOL_SIZES}"
    [[ -n "${SEED:-}" ]] && echo "  Seed: ${SEED}"
    [[ "${PROFILE_MODE}" == "true" ]] && echo "  Profiling: enabled"

    if [[ "${SCENARIO_NAME}" =~ ^tasks_update ]]; then
        echo "  tasks_update config: ID_POOL_SIZE=${ID_POOL_SIZE:-10}, RETRY_COUNT=${RETRY_COUNT:-0}, WRK_THREADS=${WRK_THREADS:-${THREADS}}"
    fi

    # Show phase-specific parameters based on RPS profile
    case "${RPS_PROFILE}" in
        step_up)
            echo "  Step-up: ${STEP_COUNT} steps, min_rps=${MIN_RPS}"
            ;;
        ramp_up_down)
            echo "  Ramp: up=${RAMP_UP_SECONDS}s, down=${RAMP_DOWN_SECONDS}s, min_rps=${MIN_RPS}"
            ;;
        burst)
            echo "  Burst: interval=${BURST_INTERVAL_SECONDS}s, duration=${BURST_DURATION_SECONDS}s, multiplier=${BURST_MULTIPLIER}x, min_rps=${MIN_RPS}"
            ;;
    esac

    # Ensure function returns success (avoid set -e exit on false && conditions)
    return 0
}

# =============================================================================
# Parameter Validation
# =============================================================================
#
# Validates all scenario parameters to ensure they are within acceptable ranges.
#
# Required parameters (error if not set or invalid):
#   - storage_mode: in_memory | postgres
#   - cache_mode: in_memory | redis | none
#   - data_scale: small | medium | large (mapped to 1e2 | 1e4 | 1e6)
#
# Optional parameters (defaults applied, validated if set):
#   - hit_rate: 0-100 (default: 50)
#   - database_pool_size: positive integer (default: 16)
#   - redis_pool_size: positive integer (default: 8)
#   - worker_threads: positive integer (default: 4)
#   - fail_rate: 0.0-1.0 (default: 0)
#   - profile: true | false (default: false)
#   - payload_variant: minimal | standard | complex | heavy (default: standard)
#   - cache_strategy: read-through | write-through | write-behind (default: read-through)
# =============================================================================

validate_scenario_parameters() {
    local has_errors=false

    echo "Validating scenario parameters..."

    # -------------------------------------------------------------------------
    # Required parameters
    # -------------------------------------------------------------------------

    # storage_mode: in_memory | postgres
    if [[ -z "${STORAGE_MODE:-}" ]]; then
        echo -e "${RED}Error: storage_mode is required but not set${NC}"
        has_errors=true
    elif [[ "${STORAGE_MODE}" != "in_memory" && "${STORAGE_MODE}" != "postgres" ]]; then
        echo -e "${RED}Error: Invalid storage_mode '${STORAGE_MODE}'. Must be: in_memory | postgres${NC}"
        has_errors=true
    fi

    # cache_mode: in_memory | redis | none
    if [[ -z "${CACHE_MODE:-}" ]]; then
        echo -e "${RED}Error: cache_mode is required but not set${NC}"
        has_errors=true
    elif [[ "${CACHE_MODE}" != "in_memory" && "${CACHE_MODE}" != "redis" && "${CACHE_MODE}" != "none" ]]; then
        echo -e "${RED}Error: Invalid cache_mode '${CACHE_MODE}'. Must be: in_memory | redis | none${NC}"
        has_errors=true
    fi

    # data_scale: 1e2 | 1e4 | 1e6
    if [[ -z "${DATA_SCALE:-}" ]]; then
        echo -e "${RED}Error: data_scale is required but not set${NC}"
        has_errors=true
    elif [[ "${DATA_SCALE}" != "1e2" && "${DATA_SCALE}" != "1e4" && "${DATA_SCALE}" != "1e6" ]]; then
        echo -e "${RED}Error: Invalid data_scale '${DATA_SCALE}'. Must be: 1e2 | 1e4 | 1e6 (mapped from small | medium | large)${NC}"
        has_errors=true
    fi

    # -------------------------------------------------------------------------
    # Optional parameters with validation
    # -------------------------------------------------------------------------

    # hit_rate: 0-100 (default: 50)
    if [[ -z "${HIT_RATE:-}" ]]; then
        export HIT_RATE="50"
    elif ! [[ "${HIT_RATE}" =~ ^[0-9]+$ ]] || [[ "${HIT_RATE}" -lt 0 ]] || [[ "${HIT_RATE}" -gt 100 ]]; then
        echo -e "${RED}Error: Invalid hit_rate '${HIT_RATE}'. Must be: 0-100${NC}"
        has_errors=true
    fi

    # database_pool_size: positive integer (default: 16)
    if [[ -z "${DATABASE_POOL_SIZE:-}" ]]; then
        export DATABASE_POOL_SIZE="16"
    elif ! [[ "${DATABASE_POOL_SIZE}" =~ ^[0-9]+$ ]] || [[ "${DATABASE_POOL_SIZE}" -lt 1 ]]; then
        echo -e "${RED}Error: Invalid database_pool_size '${DATABASE_POOL_SIZE}'. Must be: positive integer${NC}"
        has_errors=true
    fi

    # redis_pool_size: positive integer (default: 8)
    if [[ -z "${REDIS_POOL_SIZE:-}" ]]; then
        export REDIS_POOL_SIZE="8"
    elif ! [[ "${REDIS_POOL_SIZE}" =~ ^[0-9]+$ ]] || [[ "${REDIS_POOL_SIZE}" -lt 1 ]]; then
        echo -e "${RED}Error: Invalid redis_pool_size '${REDIS_POOL_SIZE}'. Must be: positive integer${NC}"
        has_errors=true
    fi

    # worker_threads: positive integer (default: 4)
    if [[ -z "${WORKER_THREADS:-}" ]]; then
        export WORKER_THREADS="4"
    elif ! [[ "${WORKER_THREADS}" =~ ^[0-9]+$ ]] || [[ "${WORKER_THREADS}" -lt 1 ]]; then
        echo -e "${RED}Error: Invalid worker_threads '${WORKER_THREADS}'. Must be: positive integer${NC}"
        has_errors=true
    fi

    # fail_rate: 0.0-1.0 (default: 0)
    if [[ -z "${FAIL_RATE:-}" ]]; then
        export FAIL_RATE="0"
    else
        # First check if it's a valid numeric format (integer or decimal)
        if ! [[ "${FAIL_RATE}" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
            echo -e "${RED}Error: Invalid fail_rate '${FAIL_RATE}'. Must be a number in range 0.0-1.0${NC}"
            has_errors=true
        else
            # Then validate range 0.0-1.0
            local is_valid_fail_rate
            is_valid_fail_rate=$(echo "${FAIL_RATE}" | awk '{
                if ($1 >= 0 && $1 <= 1) print "valid"
                else print "invalid"
            }')
            if [[ "${is_valid_fail_rate}" != "valid" ]]; then
                echo -e "${RED}Error: Invalid fail_rate '${FAIL_RATE}'. Must be in range 0.0-1.0${NC}"
                has_errors=true
            fi
        fi
    fi

    # profile: true | false (default: false)
    if [[ -z "${PROFILE:-}" ]]; then
        export PROFILE="false"
    elif [[ "${PROFILE}" != "true" && "${PROFILE}" != "false" ]]; then
        echo -e "${RED}Error: Invalid profile '${PROFILE}'. Must be: true | false${NC}"
        has_errors=true
    fi

    # retry: true | false (default: false)
    if [[ -z "${RETRY:-}" ]]; then
        export RETRY="false"
    elif [[ "${RETRY}" != "true" && "${RETRY}" != "false" ]]; then
        echo -e "${RED}Error: Invalid retry '${RETRY}'. Must be: true | false${NC}"
        has_errors=true
    fi

    # payload (PAYLOAD): minimal | standard | complex | heavy (or small | medium | large) -> small | medium | large (default: medium)
    if [[ -z "${PAYLOAD:-}" ]]; then
        export PAYLOAD="medium"
    else
        # Map payload_variant names to internal names if needed
        case "${PAYLOAD}" in
            "minimal")  export PAYLOAD="small" ;;
            "standard") export PAYLOAD="medium" ;;
            "complex"|"heavy") export PAYLOAD="large" ;;
            "small"|"medium"|"large") ;; # Already in internal format
            *)
                echo -e "${RED}Error: Invalid payload '${PAYLOAD}'. Must be: minimal | standard | complex | heavy (or small | medium | large)${NC}"
                has_errors=true
                ;;
        esac
    fi

    # cache_strategy: read-through | write-through | write-behind (default: read-through)
    if [[ -z "${CACHE_STRATEGY:-}" ]]; then
        export CACHE_STRATEGY="read-through"
    elif [[ "${CACHE_STRATEGY}" != "read-through" && "${CACHE_STRATEGY}" != "write-through" && "${CACHE_STRATEGY}" != "write-behind" ]]; then
        echo -e "${RED}Error: Invalid cache_strategy '${CACHE_STRATEGY}'. Must be: read-through | write-through | write-behind${NC}"
        has_errors=true
    fi

    # -------------------------------------------------------------------------
    # RPS control parameters validation
    # -------------------------------------------------------------------------

    # target_rps: non-negative integer (default: 0 = no rate limit)
    if [[ -n "${TARGET_RPS:-}" ]]; then
        if ! [[ "${TARGET_RPS}" =~ ^[0-9]+$ ]]; then
            echo -e "${RED}Error: Invalid target_rps '${TARGET_RPS}'. Must be: non-negative integer${NC}"
            has_errors=true
        fi
    fi

    # min_rps: positive integer (default: 10)
    if [[ -n "${MIN_RPS:-}" ]]; then
        if ! [[ "${MIN_RPS}" =~ ^[0-9]+$ ]] || [[ "${MIN_RPS}" -lt 1 ]]; then
            echo -e "${RED}Error: Invalid min_rps '${MIN_RPS}'. Must be: positive integer${NC}"
            has_errors=true
        fi
    fi

    # step_count: positive integer (default: 4)
    if [[ -n "${STEP_COUNT:-}" ]]; then
        if ! [[ "${STEP_COUNT}" =~ ^[0-9]+$ ]] || [[ "${STEP_COUNT}" -lt 1 ]]; then
            echo -e "${RED}Error: Invalid step_count '${STEP_COUNT}'. Must be: positive integer${NC}"
            has_errors=true
        fi
    fi

    # ramp_up_seconds: non-negative integer (default: 10)
    if [[ -n "${RAMP_UP_SECONDS:-}" ]]; then
        if ! [[ "${RAMP_UP_SECONDS}" =~ ^[0-9]+$ ]]; then
            echo -e "${RED}Error: Invalid ramp_up_seconds '${RAMP_UP_SECONDS}'. Must be: non-negative integer${NC}"
            has_errors=true
        fi
    fi

    # ramp_down_seconds: non-negative integer (default: 10)
    if [[ -n "${RAMP_DOWN_SECONDS:-}" ]]; then
        if ! [[ "${RAMP_DOWN_SECONDS}" =~ ^[0-9]+$ ]]; then
            echo -e "${RED}Error: Invalid ramp_down_seconds '${RAMP_DOWN_SECONDS}'. Must be: non-negative integer${NC}"
            has_errors=true
        fi
    fi

    # burst_interval_seconds: positive integer (default: 20)
    if [[ -n "${BURST_INTERVAL_SECONDS:-}" ]]; then
        if ! [[ "${BURST_INTERVAL_SECONDS}" =~ ^[0-9]+$ ]] || [[ "${BURST_INTERVAL_SECONDS}" -lt 1 ]]; then
            echo -e "${RED}Error: Invalid burst_interval_seconds '${BURST_INTERVAL_SECONDS}'. Must be: positive integer${NC}"
            has_errors=true
        fi
    fi

    # burst_duration_seconds: positive integer (default: 5)
    if [[ -n "${BURST_DURATION_SECONDS:-}" ]]; then
        if ! [[ "${BURST_DURATION_SECONDS}" =~ ^[0-9]+$ ]] || [[ "${BURST_DURATION_SECONDS}" -lt 1 ]]; then
            echo -e "${RED}Error: Invalid burst_duration_seconds '${BURST_DURATION_SECONDS}'. Must be: positive integer${NC}"
            has_errors=true
        fi
    fi

    # burst_multiplier: positive number > 0 (default: 3)
    if [[ -n "${BURST_MULTIPLIER:-}" ]]; then
        if ! [[ "${BURST_MULTIPLIER}" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
            echo -e "${RED}Error: Invalid burst_multiplier '${BURST_MULTIPLIER}'. Must be: positive number${NC}"
            has_errors=true
        else
            # Check if burst_multiplier > 0
            local is_positive
            is_positive=$(echo "${BURST_MULTIPLIER}" | awk '{ print ($1 > 0) ? "yes" : "no" }')
            if [[ "${is_positive}" != "yes" ]]; then
                echo -e "${RED}Error: burst_multiplier must be > 0 (got: ${BURST_MULTIPLIER})${NC}"
                has_errors=true
            fi
        fi
    fi

    # -------------------------------------------------------------------------
    # Profile-specific boundary condition checks
    # -------------------------------------------------------------------------

    # step_count vs duration: each step must be at least 1 second
    if [[ -n "${STEP_COUNT:-}" && -n "${DURATION_SECONDS:-}" ]]; then
        if [[ "${STEP_COUNT}" -gt "${DURATION_SECONDS}" ]]; then
            echo -e "${RED}Error: step_count (${STEP_COUNT}) > duration_seconds (${DURATION_SECONDS}). Each step must be at least 1s.${NC}"
            has_errors=true
        fi
    fi

    # ramp_up + ramp_down vs duration: total ramp time should not exceed duration
    # Note: This is a warning, not an error, as the code will auto-scale
    if [[ -n "${RAMP_UP_SECONDS:-}" && -n "${RAMP_DOWN_SECONDS:-}" && -n "${DURATION_SECONDS:-}" ]]; then
        local total_ramp=$((RAMP_UP_SECONDS + RAMP_DOWN_SECONDS))
        if [[ "${total_ramp}" -gt "${DURATION_SECONDS}" ]]; then
            echo -e "${YELLOW}Warning: ramp_up + ramp_down (${total_ramp}s) > duration (${DURATION_SECONDS}s). Will be auto-scaled.${NC}"
        fi
    fi

    # burst_interval > burst_duration: normal phase must have positive duration
    if [[ -n "${BURST_INTERVAL_SECONDS:-}" && -n "${BURST_DURATION_SECONDS:-}" ]]; then
        if [[ "${BURST_INTERVAL_SECONDS}" -le "${BURST_DURATION_SECONDS}" ]]; then
            echo -e "${RED}Error: burst_interval (${BURST_INTERVAL_SECONDS}s) must be > burst_duration (${BURST_DURATION_SECONDS}s)${NC}"
            has_errors=true
        fi
    fi

    # -------------------------------------------------------------------------
    # Calculate POOL_SIZES if not set
    # -------------------------------------------------------------------------
    if [[ -z "${POOL_SIZES:-}" ]]; then
        export POOL_SIZES=$((DATABASE_POOL_SIZE + REDIS_POOL_SIZE))
    fi

    # -------------------------------------------------------------------------
    # Exit if validation failed
    # -------------------------------------------------------------------------
    if [[ "${has_errors}" == "true" ]]; then
        echo ""
        echo -e "${RED}Validation failed. Please fix the errors above.${NC}"
        exit 1
    fi

    echo -e "${GREEN}Validation passed${NC}"
}

# Load scenario environment variables (scenario is now required)
load_scenario_env_vars "${SCENARIO_FILE}"
# Update results directory to include scenario name
RESULTS_DIR="${SCRIPT_DIR}/results/${TIMESTAMP}/${SCENARIO_NAME}"

# Apply QUICK_MODE overrides after scenario loading
# This ensures --quick flag takes precedence over scenario-defined duration
if [[ "${QUICK_MODE}" == "true" ]]; then
    echo -e "${CYAN}Quick mode enabled: overriding duration/threads/connections${NC}"
    DURATION="5s"
    THREADS="1"
    CONNECTIONS="5"
    WRK_THREADS="1"
    export DURATION THREADS CONNECTIONS WRK_THREADS
fi

# Validate all parameters
validate_scenario_parameters
echo ""

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

# =============================================================================
# wrk/wrk2 Detection and Configuration
# =============================================================================
#
# wrk2 is required for open-loop rate control (-R option).
# This ensures accurate latency measurement under target RPS.
#
# If wrk2 is not installed, the script will exit with instructions.
# =============================================================================

WRK_COMMAND=""

require_wrk2() {
    if ! command -v wrk2 &> /dev/null; then
        echo -e "${RED}ERROR: wrk2 is required for rate control but not found.${NC}"
        echo -e "${RED}Run: ./setup_wrk2.sh${NC}"
        echo ""
        echo "wrk2 provides open-loop rate control with the -R option,"
        echo "which is essential for accurate latency measurement under load."
        exit 1
    fi
    WRK_COMMAND="wrk2"
    echo -e "${GREEN}Using wrk2 for open-loop rate control${NC}"
}

# Require wrk2 for all benchmarks
require_wrk2

# Health check
echo -n "Checking API health... "
if curl -sf "${API_URL}/health" > /dev/null 2>&1; then
    echo -e "${GREEN}OK${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo ""
    echo "Error: API health check failed at ${API_URL}/health"
    echo ""
    echo "The API server must be started before running this script."
    echo ""
    echo "Recommended (via xtask with full environment integration):"
    echo "  cargo xtask bench-api --scenario ${SCENARIO_FILE:-<yaml>}"
    echo ""
    echo "Alternative (manual startup):"
    echo "  cd benches/api/docker && docker compose up -d"
    echo "  # or"
    echo "  cargo run -p task-management-benchmark-api"
    echo ""
    echo "IMPORTANT: When starting the API manually, you must pass scenario-derived"
    echo "environment variables (WORKER_THREADS, DATABASE_POOL_SIZE, REDIS_POOL_SIZE,"
    echo "STORAGE_MODE, CACHE_MODE, CACHE_STRATEGY, etc.) to the API server."
    echo "Otherwise, the benchmark results may not reflect the intended configuration."
    echo ""
    echo "Example with environment variables:"
    echo "  WORKER_THREADS=4 DATABASE_POOL_SIZE=16 REDIS_POOL_SIZE=8 \\"
    echo "    cargo run -p task-management-benchmark-api"
    echo ""
    echo "Note: When using xtask, the API is started automatically with"
    echo "      environment variables from the scenario YAML applied."
    exit 1
fi

# =============================================================================
# Cache Warmup
# =============================================================================
#
# Execute warmup requests to populate the cache before benchmark measurement.
# This is triggered when cache_state is "warm" and warmup_requests > 0.
#
# Warmup Strategy:
# - Send CACHE_WARMUP_REQUESTS GET requests to /tasks/{id}
# - Use sequential IDs from 1 to min(warmup_requests, available_tasks)
# - Silent output (only progress indicator)
# =============================================================================

run_warmup() {
    local warmup_requests=${CACHE_WARMUP_REQUESTS:-0}
    local api_port
    api_port=$(echo "${API_URL}" | sed 's/.*:\([0-9]*\).*/\1/')
    api_port=${api_port:-3002}

    if [[ "${warmup_requests}" -gt 0 ]]; then
        echo ""
        echo "=============================================="
        echo "  Cache Warmup"
        echo "=============================================="
        echo "Running warmup: ${warmup_requests} requests to populate cache..."

        local progress_interval=$((warmup_requests / 10))
        [[ "${progress_interval}" -lt 1 ]] && progress_interval=1

        for i in $(seq 1 "${warmup_requests}"); do
            # Send GET request to /tasks/{id} with sequential IDs
            # Use modulo to cycle through task IDs if warmup_requests > available tasks
            local task_id=$((i % 10000 + 1))
            curl -sf "${API_URL}/tasks/${task_id}" > /dev/null 2>&1 || true

            # Progress indicator
            if [[ $((i % progress_interval)) -eq 0 ]]; then
                local percent=$((i * 100 / warmup_requests))
                echo -n "  Progress: ${percent}% (${i}/${warmup_requests})"
                echo -e "\r"
            fi
        done
        echo "  Progress: 100% (${warmup_requests}/${warmup_requests})"
        echo -e "${GREEN}Warmup completed${NC}"
        echo ""
    fi
}

# Run warmup if configured
run_warmup

# Create results directory
mkdir -p "${RESULTS_DIR}"

# Record benchmark start time for validation overhead calculation (REQ-PROFILE-JSON-001)
# Use millisecond precision to avoid TOTAL_TIME=0 for short executions
# Try GNU date first (Linux), fallback to Python for macOS/BSD
get_timestamp_ms() {
    # Try GNU date (check if output is numeric milliseconds)
    local test_output
    test_output=$(date +%s%3N 2>&1)
    # Check if output is a valid number (all digits)
    if [[ "${test_output}" =~ ^[0-9]+$ ]]; then
        # GNU date (Linux) - output is numeric milliseconds
        echo "${test_output}"
    elif command -v python3 >/dev/null 2>&1; then
        # Python fallback (macOS/BSD) with error handling and validation
        local python_output
        python_output=$(python3 -c "import time; print(int(time.time() * 1000))" 2>/dev/null)
        # Validate python output is numeric
        if [[ "${python_output}" =~ ^[0-9]+$ ]]; then
            echo "${python_output}"
        else
            # Python failed or returned non-numeric, use seconds * 1000 as fallback
            echo "$(($(date +%s) * 1000))"
        fi
    else
        # Last resort: seconds * 1000
        echo "$(($(date +%s) * 1000))"
    fi
}
BENCHMARK_START_TIME=$(get_timestamp_ms)
BENCHMARK_TIME_UNIT="ms"

# Summary file
SUMMARY_FILE="${RESULTS_DIR}/summary.txt"
echo "Benchmark Results - $(date)" > "${SUMMARY_FILE}"
echo "================================" >> "${SUMMARY_FILE}"
echo "" >> "${SUMMARY_FILE}"

# =============================================================================
# Generate meta.json
# =============================================================================

# Helper: Convert latency string (e.g., "12.5ms", "500us", "1.2s") to milliseconds number
# Returns empty string if input is empty or cannot be parsed
parse_latency_to_ms() {
    local value="$1"

    if [[ -z "${value}" || "${value}" == "0" ]]; then
        echo ""
        return
    fi

    local num unit
    num=$(echo "${value}" | sed 's/[^0-9.]//g')
    unit=$(echo "${value}" | sed 's/[0-9.]//g')

    if [[ -z "${num}" ]]; then
        echo ""
        return
    fi

    case "${unit}" in
        us) awk "BEGIN {printf \"%.4f\", ${num} / 1000}" ;;
        ms) echo "${num}" ;;
        s)  awk "BEGIN {printf \"%.4f\", ${num} * 1000}" ;;
        *)  echo "${num}" ;;
    esac
}

# Helper: Format error_rate value for JSON (number or null)
# Per JSON schema: error_rate must be in range [0, 1] or null
# Handles bc output format (.123456 -> 0.123456) and clamps to [0, 1]
format_error_rate_json() {
    local value="$1"
    local total_requests="${2:-0}"

    # Validate total_requests is a positive integer (avoid octal interpretation with 10#)
    # Pattern: 0 or positive integer without leading zeros (except "0" itself)
    if ! [[ "${total_requests}" =~ ^(0|[1-9][0-9]*)$ ]]; then
        echo "null"
        return
    fi

    # Use 10# prefix to force decimal interpretation and check > 0
    if ! ((10#${total_requests} > 0)); then
        echo "null"
        return
    fi

    # Empty value returns null
    if [[ -z "${value}" ]]; then
        echo "null"
        return
    fi

    # Validate numeric format for value
    if ! [[ "${value}" =~ ^-?[0-9]*\.?[0-9]+$ ]]; then
        echo "null"
        return
    fi

    # Normalize and clamp to [0, 1] using awk
    # awk's printf "%.6f" automatically includes leading zero (0.123, not .123)
    awk -v val="${value}" 'BEGIN {
        rate = val + 0
        if (rate < 0) rate = 0
        if (rate > 1) rate = 1
        printf "%.6f", rate
    }'
}

# Helper: Validate latency value is valid (non-empty, numeric, positive, non-zero)
# Returns 0 (success) if valid, 1 (failure) if invalid
# Per REQ-PROFILE-JSON-002: 0ms latency is invalid (physically impossible)
is_valid_latency() {
    local value="$1"

    # Empty or null values are invalid
    [[ -z "${value}" || "${value}" == "null" ]] && return 1

    # Validate numeric format: optional minus, digits, optional decimal point and digits
    # Reject non-numeric values like "N/A", "1ms", "nan", "inf"
    # Note: Rejects exponential notation (1e-3) and plus signs (+1.23) as wrk outputs
    # standard decimal notation only (e.g., "1.23", "0.45")
    [[ ! "${value}" =~ ^-?[0-9]*\.?[0-9]+$ ]] && return 1

    # Check if value is 0 (0, 0.0, 0.00, etc.)
    awk -v val="${value}" 'BEGIN { exit (val + 0 == 0) ? 1 : 0 }' || return 1

    # Check if value is negative
    awk -v val="${value}" 'BEGIN { exit (val + 0 < 0) ? 1 : 0 }' || return 1

    return 0
}

# Helper: Format latency value for JSON (number or null)
# v3: Use null for missing latency values, NOT 0
# Per REQ-PROFILE-JSON-002: latency of 0 is considered "unavailable" and converted to null.
# This is because a true 0ms latency is physically impossible and indicates measurement failure.
format_latency_json() {
    local value="$1"

    # Use is_valid_latency for unified validation logic
    if ! is_valid_latency "${value}"; then
        echo "null"
        return
    fi

    # Normalize the numeric format to ensure valid JSON
    # Use awk to output with proper formatting (handles .123 -> 0.123)
    awk -v val="${value}" 'BEGIN { printf "%.6f", val + 0 }'
}

# Compute error_rate from HTTP and socket errors
# Returns "0.0" for requests=0, formatted rate string otherwise
# Usage: compute_error_rate <requests> <http_4xx> <http_5xx> <socket_errors_total>
compute_error_rate() {
    local requests="${1:-0}"
    local http_4xx="${2:-0}"
    local http_5xx="${3:-0}"
    local socket_errors_total="${4:-0}"

    # requests=0 -> 0.0 (per requirement REQ-MET-P3-002 L80)
    if [[ ! "${requests}" =~ ^[0-9]+$ ]] || (( 10#${requests} == 0 )); then
        echo "0.0"
        return
    fi

    local total_errors=$(( 10#${http_4xx:-0} + 10#${http_5xx:-0} + 10#${socket_errors_total:-0} ))
    awk -v errors="${total_errors}" -v total="${requests}" 'BEGIN {
        rate = errors / total
        if (rate < 0) rate = 0
        if (rate > 1) rate = 1
        printf "%.6f", rate
    }'
}

generate_meta_json() {
    local result_file="$1"
    local script_name="$2"
    local meta_file="${RESULTS_DIR}/meta.json"
    local lua_metrics_file="${RESULTS_DIR}/lua_metrics.json"

    # Parse wrk output for metrics
    # Note: Use anchored patterns to avoid matching percentages in other contexts
    # (e.g., "75.99%" in Latency line should not match "99%" pattern)
    local rps avg_latency_raw p50_raw p90_raw p95_raw p99_raw total_requests
    rps=$(grep "Requests/sec:" "${result_file}" 2>/dev/null | awk '{print $2}' || echo "0")
    avg_latency_raw=$(grep "Latency" "${result_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "")
    p50_raw=$(grep -E "^[[:space:]]+50[.0-9]*%" "${result_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "")
    p90_raw=$(grep -E "^[[:space:]]+90[.0-9]*%" "${result_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "")
    p95_raw=$(grep -E "^[[:space:]]+95[.0-9]*%" "${result_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "")
    p99_raw=$(grep -E "^[[:space:]]+99[.0-9]*%" "${result_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "")
    total_requests=$(grep -m1 "requests in" "${result_file}" 2>/dev/null | awk '{print $1}' || echo "0")
    [[ ! "${total_requests}" =~ ^[0-9]+$ ]] && total_requests=0

    # v3: Convert latency strings to milliseconds numbers
    local avg_latency_ms p50_ms p90_ms p95_ms p99_ms
    avg_latency_ms=$(parse_latency_to_ms "${avg_latency_raw}")
    p50_ms=$(parse_latency_to_ms "${p50_raw}")
    p90_ms=$(parse_latency_to_ms "${p90_raw}")
    p95_ms=$(parse_latency_to_ms "${p95_raw}")
    p99_ms=$(parse_latency_to_ms "${p99_raw}")

    # v3: Format for JSON (null for missing values)
    local avg_latency_json p50_json p90_json p95_json p99_json
    avg_latency_json=$(format_latency_json "${avg_latency_ms}")
    p50_json=$(format_latency_json "${p50_ms}")
    p90_json=$(format_latency_json "${p90_ms}")
    p95_json=$(format_latency_json "${p95_ms}")
    p99_json=$(format_latency_json "${p99_ms}")

    # Parse socket errors breakdown
    local connect_err=0 read_err=0 write_err=0 timeout_err=0 socket_errors=0
    if grep -q "Socket errors:" "${result_file}" 2>/dev/null; then
        connect_err=$(grep "Socket errors:" "${result_file}" | sed 's/.*connect \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
        read_err=$(grep "Socket errors:" "${result_file}" | sed 's/.*read \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
        write_err=$(grep "Socket errors:" "${result_file}" | sed 's/.*write \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
        timeout_err=$(grep "Socket errors:" "${result_file}" | sed 's/.*timeout \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
        socket_errors=$((connect_err + read_err + write_err + timeout_err))
    fi

    # Parse HTTP errors from wrk output ("Non-2xx or 3xx responses: N")
    local http_errors_from_wrk=0
    if grep -q "Non-2xx or 3xx responses:" "${result_file}" 2>/dev/null; then
        http_errors_from_wrk=$(grep -m1 "Non-2xx or 3xx responses:" "${result_file}" | awk '{print $NF}' 2>/dev/null || echo "0")
        [[ ! "${http_errors_from_wrk}" =~ ^[0-9]+$ ]] && http_errors_from_wrk=0
    fi

    # HTTP error counts: initialized to wrk values, updated from lua_metrics if available
    # wrk doesn't distinguish 4xx vs 5xx, so http_4xx/http_5xx remain 0 unless lua_metrics provides them
    local http_4xx=0 http_5xx=0 http_status_total=${http_errors_from_wrk}

    # v3: http_status distribution (from lua_metrics.json)
    local http_status_json="{}"

    # v3: retries count (from lua_metrics.json)
    local retries=0

    # v3: error_rate will be calculated after lua_metrics.json is processed
    local error_rate

    # Collect environment information
    local os_name cpu_cores memory_gb rust_version
    os_name="$(uname -s) $(uname -r)"
    if [[ "$(uname)" == "Darwin" ]]; then
        cpu_cores=$(sysctl -n hw.ncpu 2>/dev/null || echo "0")
        memory_gb=$(( $(sysctl -n hw.memsize 2>/dev/null || echo "0") / 1073741824 ))
    else
        cpu_cores=$(nproc 2>/dev/null || echo "0")
        memory_gb=$(( $(grep MemTotal /proc/meminfo 2>/dev/null | awk '{print $2}' || echo "0") / 1048576 ))
    fi
    rust_version=$(rustc --version 2>/dev/null | awk '{print $2}' || echo "unknown")

    # Default cache metrics (will be updated from lua_metrics if available)
    local cache_hit_rate="null" cache_misses="null" cache_hits="null"

    # Check for profiling files (use raw strings, will be converted to JSON via jq)
    local perf_data_path_raw="" flamegraph_path_raw="" pprof_path_raw=""
    if [[ "${PROFILE_MODE}" == "true" ]]; then
        [[ -f "${RESULTS_DIR}/perf.data" ]] && perf_data_path_raw="perf.data"
        [[ -f "${RESULTS_DIR}/flamegraph.svg" ]] && flamegraph_path_raw="flamegraph.svg"
        [[ -f "${RESULTS_DIR}/pprof.pb.gz" ]] && pprof_path_raw="pprof.pb.gz"
    fi

    # Try to read lua_metrics.json if it exists and is valid JSON
    if [[ -f "${lua_metrics_file}" ]] && command -v jq &> /dev/null && jq -e . "${lua_metrics_file}" &>/dev/null; then
        # Read http_status distribution from lua_metrics (REQ-PIPELINE-003)
        # This is the primary source for HTTP status codes
        if jq -e '.http_status | type == "object" and length > 0' "${lua_metrics_file}" &>/dev/null; then
            http_status_json=$(jq -c '.http_status // {}' "${lua_metrics_file}" 2>/dev/null || echo "{}")

            # Calculate 4xx and 5xx totals from http_status
            local lua_4xx=0 lua_5xx=0
            for code in $(jq -r '.http_status | keys[]' "${lua_metrics_file}" 2>/dev/null); do
                if [[ "${code}" =~ ^4[0-9][0-9]$ ]]; then
                    local count
                    count=$(jq -r ".http_status.\"${code}\" // 0" "${lua_metrics_file}" 2>/dev/null)
                    [[ "${count}" =~ ^[0-9]+$ ]] && lua_4xx=$((lua_4xx + count))
                elif [[ "${code}" =~ ^5[0-9][0-9]$ ]]; then
                    local count
                    count=$(jq -r ".http_status.\"${code}\" // 0" "${lua_metrics_file}" 2>/dev/null)
                    [[ "${count}" =~ ^[0-9]+$ ]] && lua_5xx=$((lua_5xx + count))
                fi
            done

            local lua_total=$((lua_4xx + lua_5xx))

            # Use lua_metrics breakdown if http_status is non-empty
            # Even if errors are 0, this ensures consistency with http_status
            http_4xx="${lua_4xx}"
            http_5xx="${lua_5xx}"
            http_status_total=${lua_total}
        elif jq -e '.errors.status | type == "object" and (has("4xx") or has("5xx"))' "${lua_metrics_file}" &>/dev/null; then
            # Fallback: Check if errors.status is an object with 4xx or 5xx keys (legacy v3 schema)
            local lua_4xx_raw lua_5xx_raw
            lua_4xx_raw=$(jq -r '.errors.status["4xx"] // 0' "${lua_metrics_file}" 2>/dev/null)
            lua_5xx_raw=$(jq -r '.errors.status["5xx"] // 0' "${lua_metrics_file}" 2>/dev/null)

            # Convert to integer (bash truncates decimals, handles non-numeric as 0)
            local lua_4xx="${lua_4xx_raw%%.*}"
            local lua_5xx="${lua_5xx_raw%%.*}"
            [[ ! "${lua_4xx}" =~ ^[0-9]+$ ]] && lua_4xx=0
            [[ ! "${lua_5xx}" =~ ^[0-9]+$ ]] && lua_5xx=0

            local lua_total=$((lua_4xx + lua_5xx))

            # Use lua_metrics breakdown if available
            # Accept even if errors are 0 for consistency
            http_4xx="${lua_4xx}"
            http_5xx="${lua_5xx}"
            http_status_total=${lua_total}

            # Try to get http_status from legacy format
            http_status_json=$(jq -c '.http_status // {}' "${lua_metrics_file}" 2>/dev/null || echo "{}")
        fi

        # Extract cache metrics with type normalization (tonumber? ensures numeric type)
        local cache_hit_rate_raw cache_misses_raw cache_hits_raw
        cache_hit_rate_raw=$(jq -r '(.cache.hit_rate | tonumber?) // null' "${lua_metrics_file}" 2>/dev/null || echo "null")
        cache_misses_raw=$(jq -r '(.cache.cache_misses | tonumber?) // null' "${lua_metrics_file}" 2>/dev/null || echo "null")
        cache_hits_raw=$(jq -r '(.cache.cache_hits | tonumber?) // null' "${lua_metrics_file}" 2>/dev/null || echo "null")

        # Validate extracted values (handle "nan", "inf", non-numeric strings)
        if [[ "${cache_hit_rate_raw}" == "null" || "${cache_hit_rate_raw}" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
            cache_hit_rate="${cache_hit_rate_raw}"
        else
            cache_hit_rate="null"
        fi

        if [[ "${cache_misses_raw}" == "null" || "${cache_misses_raw}" =~ ^[0-9]+$ ]]; then
            cache_misses="${cache_misses_raw}"
        else
            cache_misses="null"
        fi

        if [[ "${cache_hits_raw}" == "null" || "${cache_hits_raw}" =~ ^[0-9]+$ ]]; then
            cache_hits="${cache_hits_raw}"
        else
            cache_hits="null"
        fi

        # v3: Get retries count from lua_metrics
        local retries_raw
        retries_raw=$(jq -r '.retries // 0' "${lua_metrics_file}" 2>/dev/null || echo "0")
        retries="${retries_raw%%.*}"
        [[ ! "${retries}" =~ ^[0-9]+$ ]] && retries=0
    fi

    # v3: Calculate error_rate using single-source formula (REQ-MET-P3-002)
    # error_rate = (http_4xx + http_5xx + socket_errors) / requests
    # http_4xx, http_5xx are already extracted from lua_metrics.json above
    # Socket errors are reported separately in errors.socket_errors
    error_rate=$(compute_error_rate "${total_requests}" "${http_4xx}" "${http_5xx}" "${socket_errors}")
    echo -e "${CYAN}Computed error_rate: ${error_rate}${NC}" >&2

    # Get wrk output filename
    local wrk_output_filename
    wrk_output_filename=$(basename "${result_file}")

    # v3: Parse duration to seconds (remove 's' suffix)
    # Use MERGED_DURATION if available (from phased execution)
    local duration_seconds
    if [[ -n "${MERGED_DURATION:-}" ]]; then
        duration_seconds="${MERGED_DURATION}"
    else
        duration_seconds=$(echo "${DURATION}" | sed 's/s$//')
        if [[ -z "${duration_seconds}" || ! "${duration_seconds}" =~ ^[0-9]+$ ]]; then
            duration_seconds=30
        fi
    fi

    # Override metrics with merged values if available (from phased execution)
    # Track validation status for later use in summary.txt
    local invalid_total_requests="false"

    if [[ -n "${MERGED_RPS:-}" ]]; then
        rps="${MERGED_RPS}"
    fi
    if [[ -n "${MERGED_REQUESTS:-}" ]]; then
        # Validate MERGED_REQUESTS is numeric before assignment
        if [[ "${MERGED_REQUESTS}" =~ ^[0-9]+$ ]]; then
            total_requests="${MERGED_REQUESTS}"
        else
            echo -e "${RED}ERROR: MERGED_REQUESTS is not numeric: ${MERGED_REQUESTS}${NC}" >&2
            invalid_total_requests="true"
            total_requests=0
            # Reset error_rate to null when total_requests is invalid
            # to maintain data consistency in meta.json
            error_rate="null"
        fi
    fi
    if [[ -n "${MERGED_P50:-}" ]]; then
        # MERGED_P50 is already in milliseconds, format for JSON
        p50_json=$(format_latency_json "${MERGED_P50}")
        p50_ms="${MERGED_P50}"
    fi
    if [[ -n "${MERGED_P90:-}" ]]; then
        # MERGED_P90 is already in milliseconds, format for JSON
        p90_json=$(format_latency_json "${MERGED_P90}")
        p90_ms="${MERGED_P90}"
    fi
    if [[ -n "${MERGED_P95:-}" ]]; then
        # MERGED_P95 is already in milliseconds, format for JSON
        p95_json=$(format_latency_json "${MERGED_P95}")
        p95_ms="${MERGED_P95}"
    fi
    if [[ -n "${MERGED_P99:-}" ]]; then
        # MERGED_P99 is already in milliseconds, format for JSON
        p99_json=$(format_latency_json "${MERGED_P99}")
        p99_ms="${MERGED_P99}"
    fi
    if [[ -n "${MERGED_SOCKET_ERRORS:-}" ]]; then
        socket_errors="${MERGED_SOCKET_ERRORS}"
    fi
    if [[ -n "${MERGED_ERROR_RATE:-}" ]]; then
        # Normalize MERGED_ERROR_RATE using format_error_rate_json
        # This handles bc output format (.123 -> 0.123) and clamps to [0, 1]
        # Always apply normalized result to maintain consistency with MERGED_REQUESTS
        # If total_requests is 0/invalid, error_rate becomes null (as expected)
        error_rate=$(format_error_rate_json "${MERGED_ERROR_RATE}" "${total_requests}")
    fi

    # Validate percentiles (REQ-PROFILE-JSON-002)
    # Validation is performed after MERGED values are applied to ensure correct values are checked
    # Check if MERGED_REQUESTS was invalid before validation
    if [[ "${invalid_total_requests}" == "true" ]]; then
        echo "Benchmark failed: invalid MERGED_REQUESTS value (not numeric)" >> "${SUMMARY_FILE:-/dev/null}"
        return 1
    fi

    # Store validation result to capture missing/invalid percentile details
    # Check for missing, zero, or invalid values (per REQ-PROFILE-JSON-002)
    local missing_percentiles=()
    is_valid_latency "${p50_ms}" || missing_percentiles+=("p50")
    is_valid_latency "${p90_ms}" || missing_percentiles+=("p90")
    is_valid_latency "${p99_ms}" || missing_percentiles+=("p99")

    if ! validate_required_percentiles "${p50_ms}" "${p90_ms}" "${p99_ms}" "${total_requests}"; then
        # Determine failure reason for summary.txt
        if [[ ! "${total_requests}" =~ ^(0|[1-9][0-9]*)$ ]]; then
            echo "Benchmark failed: invalid total_requests value (${total_requests})" >> "${SUMMARY_FILE:-/dev/null}"
        elif [[ ${#missing_percentiles[@]} -gt 0 ]]; then
            echo "Benchmark failed: percentile data missing/zero/invalid (${missing_percentiles[*]})" >> "${SUMMARY_FILE:-/dev/null}"
        else
            echo "Benchmark failed: percentile validation error" >> "${SUMMARY_FILE:-/dev/null}"
        fi
        return 1
    fi

    # Prepare phased execution metadata using jq for safe JSON generation
    local phased_execution_json="null"
    if [[ -n "${MERGED_PHASE_COUNT:-}" && "${MERGED_PHASE_COUNT}" -gt 1 ]]; then
        phased_execution_json=$(jq -n \
            --argjson phase_count "${MERGED_PHASE_COUNT}" \
            --arg profile "${RPS_PROFILE:-steady}" \
            '{
                "enabled": true,
                "phase_count": $phase_count,
                "profile": $profile
            }')
    fi

    # ==========================================================================
    # Fetch applied_env from /debug/config endpoint (ENV-REQ-030)
    # ==========================================================================
    # The /debug/config endpoint is only available when ENABLE_DEBUG_ENDPOINTS=true.
    # If available, we compare scenario_requested values with actual applied values
    # to detect any configuration mismatches.
    #
    # Security: The /debug/config endpoint does not expose sensitive values
    # (DATABASE_URL, REDIS_URL, etc.)
    # ==========================================================================
    # env_mismatch is "null" when /debug/config is unavailable (unknown state),
    # "true" when mismatch detected, "false" when comparison succeeded with no mismatch.
    local scenario_requested_json applied_env_json env_mismatch="null"
    local applied_worker_threads="null" applied_database_pool_size="null" applied_redis_pool_size="null"
    local applied_storage_mode="null" applied_cache_mode="null"

    # Build scenario_requested from current environment variables using jq
    # Validate numeric values before passing to --argjson
    local validated_worker_threads="null"
    local validated_database_pool_size="null"
    local validated_redis_pool_size="null"

    if [[ -n "${WORKER_THREADS:-}" && "${WORKER_THREADS}" =~ ^[0-9]+$ ]]; then
        validated_worker_threads="${WORKER_THREADS}"
    fi
    if [[ -n "${DATABASE_POOL_SIZE:-}" && "${DATABASE_POOL_SIZE}" =~ ^[0-9]+$ ]]; then
        validated_database_pool_size="${DATABASE_POOL_SIZE}"
    fi
    if [[ -n "${REDIS_POOL_SIZE:-}" && "${REDIS_POOL_SIZE}" =~ ^[0-9]+$ ]]; then
        validated_redis_pool_size="${REDIS_POOL_SIZE}"
    fi

    scenario_requested_json=$(jq -n \
        --argjson worker_threads "${validated_worker_threads}" \
        --argjson database_pool_size "${validated_database_pool_size}" \
        --argjson redis_pool_size "${validated_redis_pool_size}" \
        '{
            "worker_threads": $worker_threads,
            "database_pool_size": $database_pool_size,
            "redis_pool_size": $redis_pool_size
        }')

    # Try to fetch /debug/config from the API
    local debug_config_response
    if debug_config_response=$(curl -s -f "${API_URL}/debug/config" 2>/dev/null); then
        # Parse the response if jq is available
        if command -v jq &>/dev/null && echo "${debug_config_response}" | jq -e . &>/dev/null; then
            # Use jq to normalize types (convert string numbers to numbers, preserve null)
            # tonumber? converts strings to numbers, or passes through if already a number
            applied_worker_threads=$(echo "${debug_config_response}" | jq '(.worker_threads | tonumber?) // null')
            applied_database_pool_size=$(echo "${debug_config_response}" | jq '(.database_pool_size | tonumber?) // null')
            applied_redis_pool_size=$(echo "${debug_config_response}" | jq '(.redis_pool_size | tonumber?) // null')
            # For string fields, use -r but handle null specially
            local storage_mode_raw cache_mode_raw
            storage_mode_raw=$(echo "${debug_config_response}" | jq -r '.storage_mode // empty')
            cache_mode_raw=$(echo "${debug_config_response}" | jq -r '.cache_mode // empty')

            # Build applied_env JSON using jq for safe generation
            applied_env_json=$(jq -n \
                --argjson worker_threads "${applied_worker_threads}" \
                --argjson database_pool_size "${applied_database_pool_size}" \
                --argjson redis_pool_size "${applied_redis_pool_size}" \
                --arg storage_mode "${storage_mode_raw}" \
                --arg cache_mode "${cache_mode_raw}" \
                '{
                    "worker_threads": $worker_threads,
                    "database_pool_size": $database_pool_size,
                    "redis_pool_size": $redis_pool_size,
                    "storage_mode": (if $storage_mode == "" then null else $storage_mode end),
                    "cache_mode": (if $cache_mode == "" then null else $cache_mode end)
                }')

            # Detect mismatch between scenario_requested and applied_env
            # Compare only the fields that are in both (worker_threads, database_pool_size, redis_pool_size)
            local req_wt="${WORKER_THREADS:-null}"
            local req_dbp="${DATABASE_POOL_SIZE:-null}"
            local req_rp="${REDIS_POOL_SIZE:-null}"

            # Normalize "null" string to actual null for comparison
            [[ "${applied_worker_threads}" == "null" ]] && applied_worker_threads="null"
            [[ "${applied_database_pool_size}" == "null" ]] && applied_database_pool_size="null"
            [[ "${applied_redis_pool_size}" == "null" ]] && applied_redis_pool_size="null"

            # Successfully fetched /debug/config, so we can determine mismatch status.
            # Default to false (no mismatch), then check for mismatches.
            env_mismatch="false"

            # Check for mismatches (only when both values are non-null)
            if [[ "${req_wt}" != "null" && "${applied_worker_threads}" != "null" && "${req_wt}" != "${applied_worker_threads}" ]]; then
                env_mismatch="true"
            elif [[ "${req_dbp}" != "null" && "${applied_database_pool_size}" != "null" && "${req_dbp}" != "${applied_database_pool_size}" ]]; then
                env_mismatch="true"
            elif [[ "${req_rp}" != "null" && "${applied_redis_pool_size}" != "null" && "${req_rp}" != "${applied_redis_pool_size}" ]]; then
                env_mismatch="true"
            fi
        else
            # jq not available or invalid JSON, set applied_env to null
            applied_env_json="null"
        fi
    else
        # /debug/config not available (ENABLE_DEBUG_ENDPOINTS=false or API down)
        applied_env_json="null"
    fi

    # Generate meta.json with schema v3.0
    # Per REQ-PROFILE-JSON-001: Use jq for JSON generation to prevent injection attacks
    # and ensure valid JSON format (proper escaping of special characters)

    # Prepare timestamp
    local timestamp
    timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    # Prepare lua_metrics file reference (use raw string, will be converted to JSON via jq)
    local lua_metrics_ref_raw=""
    if [[ -f "${lua_metrics_file}" ]]; then
        lua_metrics_ref_raw="lua_metrics.json"
    fi

    # Validate numeric parameters before passing to --argjson
    # This prevents jq failures when receiving non-numeric or empty values
    [[ ! "${THREADS:-}" =~ ^[0-9]+$ ]] && THREADS="2"
    [[ ! "${CONNECTIONS:-}" =~ ^[0-9]+$ ]] && CONNECTIONS="10"
    [[ ! "${duration_seconds:-}" =~ ^[0-9]+$ ]] && duration_seconds="30"
    [[ ! "${total_requests:-}" =~ ^[0-9]+$ ]] && total_requests="0"
    [[ ! "${cpu_cores:-}" =~ ^[0-9]+$ ]] && cpu_cores="0"
    [[ ! "${memory_gb:-}" =~ ^[0-9]+$ ]] && memory_gb="0"
    [[ ! "${retries:-}" =~ ^[0-9]+$ ]] && retries="0"
    [[ ! "${connect_err:-}" =~ ^[0-9]+$ ]] && connect_err="0"
    [[ ! "${read_err:-}" =~ ^[0-9]+$ ]] && read_err="0"
    [[ ! "${write_err:-}" =~ ^[0-9]+$ ]] && write_err="0"
    [[ ! "${timeout_err:-}" =~ ^[0-9]+$ ]] && timeout_err="0"
    [[ ! "${socket_errors:-}" =~ ^[0-9]+$ ]] && socket_errors="0"
    [[ ! "${http_4xx:-}" =~ ^[0-9]+$ ]] && http_4xx="0"
    [[ ! "${http_5xx:-}" =~ ^[0-9]+$ ]] && http_5xx="0"
    [[ ! "${http_status_total:-}" =~ ^[0-9]+$ ]] && http_status_total="0"

    # Validate optional numeric parameters (POOL_SIZES, SEED)
    # These default to null if not set or invalid
    local validated_pool_sizes="null"
    local validated_seed="null"
    if [[ -n "${POOL_SIZES:-}" && "${POOL_SIZES}" =~ ^[0-9]+$ ]]; then
        validated_pool_sizes="${POOL_SIZES}"
    fi
    if [[ -n "${SEED:-}" && "${SEED}" =~ ^[0-9]+$ ]]; then
        validated_seed="${SEED}"
    fi

    # Validate floating-point parameters (rps, error_rate)
    # These can be null, integers, or floats
    local validated_rps="null"
    local validated_error_rate="null"

    if [[ -n "${rps:-}" && "${rps}" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
        validated_rps="${rps}"
    fi

    if [[ -n "${error_rate:-}" && "${error_rate}" != "null" ]]; then
        if [[ "${error_rate}" =~ ^[0-9]+(\.[0-9]+)?$ ]]; then
            validated_error_rate="${error_rate}"
        fi
    fi

    # Use jq -n to generate valid JSON with proper escaping
    # All string values are passed via --arg to ensure proper escaping
    # Numeric values are passed via --argjson to preserve their type
    jq -n \
        --arg version "3.0" \
        --arg scenario_name "${SCENARIO_NAME:-${script_name}}" \
        --arg storage_mode "${STORAGE_MODE:-}" \
        --arg cache_mode "${CACHE_MODE:-}" \
        --arg data_scale "${DATA_SCALE:-1e4}" \
        --arg payload_variant "${PAYLOAD:-medium}" \
        --arg rps_profile "${RPS_PROFILE:-steady}" \
        --argjson hit_rate "${HIT_RATE:-null}" \
        --arg cache_strategy "${CACHE_STRATEGY:-read-through}" \
        --argjson fail_injection "${FAIL_RATE:-null}" \
        --argjson retry "${RETRY:-false}" \
        --arg endpoint "${ENDPOINT:-mixed}" \
        --arg timestamp "${timestamp}" \
        --argjson threads "${THREADS}" \
        --argjson connections "${CONNECTIONS}" \
        --argjson duration_seconds "${duration_seconds}" \
        --argjson worker_threads "${validated_worker_threads}" \
        --argjson pool_sizes "${validated_pool_sizes}" \
        --argjson database_pool_size "${validated_database_pool_size}" \
        --argjson redis_pool_size "${validated_redis_pool_size}" \
        --argjson seed "${validated_seed}" \
        --argjson total_requests "${total_requests:-0}" \
        --argjson error_rate "${validated_error_rate}" \
        --argjson rps "${validated_rps}" \
        --argjson avg_latency "${avg_latency_json}" \
        --argjson p50 "${p50_json}" \
        --argjson p90 "${p90_json}" \
        --argjson p95 "${p95_json}" \
        --argjson p99 "${p99_json}" \
        --argjson http_status "${http_status_json}" \
        --argjson retries "${retries}" \
        --argjson connect_err "${connect_err:-0}" \
        --argjson read_err "${read_err:-0}" \
        --argjson write_err "${write_err:-0}" \
        --argjson timeout_err "${timeout_err:-0}" \
        --argjson socket_errors "${socket_errors:-0}" \
        --argjson http_4xx "${http_4xx:-0}" \
        --argjson http_5xx "${http_5xx:-0}" \
        --argjson http_status_total "${http_status_total:-0}" \
        --argjson cache_hit_rate "${cache_hit_rate}" \
        --argjson cache_misses "${cache_misses}" \
        --argjson cache_hits "${cache_hits}" \
        --arg perf_data_path "${perf_data_path_raw}" \
        --arg flamegraph_path "${flamegraph_path_raw}" \
        --arg pprof_path "${pprof_path_raw}" \
        --arg wrk_output_filename "${wrk_output_filename}" \
        --arg lua_metrics_ref "${lua_metrics_ref_raw}" \
        --arg api_url "${API_URL}" \
        --arg rust_version "${rust_version}" \
        --arg os_name "${os_name}" \
        --argjson cpu_cores "${cpu_cores}" \
        --argjson memory_gb "${memory_gb}" \
        --argjson phased_execution "${phased_execution_json}" \
        --argjson scenario_requested "${scenario_requested_json}" \
        --argjson applied_env "${applied_env_json}" \
        --argjson env_mismatch "${env_mismatch}" \
        '{
  "version": $version,
  "scenario": {
    "name": $scenario_name,
    "storage_mode": (if $storage_mode == "" then "in_memory" else $storage_mode end),
    "cache_mode": (if $cache_mode == "" then "none" else $cache_mode end),
    "data_scale": $data_scale,
    "payload_variant": $payload_variant,
    "rps_profile": $rps_profile,
    "hit_rate": $hit_rate,
    "cache_strategy": $cache_strategy,
    "fail_injection": $fail_injection,
    "retry": $retry,
    "endpoint": $endpoint
  },
  "execution": {
    "timestamp": $timestamp,
    "threads": $threads,
    "connections": $connections,
    "duration_seconds": $duration_seconds,
    "worker_threads": $worker_threads,
    "pool_sizes": $pool_sizes,
    "database_pool_size": $database_pool_size,
    "redis_pool_size": $redis_pool_size,
    "seed": $seed
  },
  "results": {
    "requests": $total_requests,
    "duration_seconds": $duration_seconds,
    "error_rate": $error_rate,
    "rps": $rps,
    "latency_ms": {
      "avg": $avg_latency,
      "p50": $p50,
      "p90": $p90,
      "p95": $p95,
      "p99": $p99
    },
    "http_status": $http_status,
    "retries": $retries
  },
  "errors": {
    "socket_errors": {
      "connect": $connect_err,
      "read": $read_err,
      "write": $write_err,
      "timeout": $timeout_err,
      "total": $socket_errors
    },
    "http_4xx": $http_4xx,
    "http_5xx": $http_5xx,
    "http_status_total": $http_status_total,
    "error_rate_includes_409": true
  },
  "cache": {
    "hit_rate": $cache_hit_rate,
    "misses": $cache_misses,
    "hits": $cache_hits
  },
  "profiling": {
    "perf_data": (if $perf_data_path == "" then null else $perf_data_path end),
    "flamegraph": (if $flamegraph_path == "" then null else $flamegraph_path end),
    "pprof": (if $pprof_path == "" then null else $pprof_path end)
  },
  "files": {
    "wrk_output": $wrk_output_filename,
    "lua_metrics": (if $lua_metrics_ref == "" then null else $lua_metrics_ref end)
  },
  "environment": {
    "api_url": $api_url,
    "rust_version": $rust_version,
    "os": $os_name,
    "cpu_cores": $cpu_cores,
    "memory_gb": $memory_gb
  },
  "phased_execution": $phased_execution,
  "scenario_requested": $scenario_requested,
  "applied_env": $applied_env,
  "env_mismatch": $env_mismatch
}' > "${meta_file}"

    echo -e "${GREEN}meta.json generated (v3.0) using jq${NC}"

    # Status coverage verification (REQ-MET-P3-001)
    if command -v jq &> /dev/null && [[ -f "${meta_file}" ]]; then
        local status_sum requests_count coverage
        status_sum=$(jq '[.results.http_status | to_entries[] | .value] | add // 0' "${meta_file}" 2>/dev/null || echo "0")
        requests_count=$(jq '.results.requests // 0' "${meta_file}" 2>/dev/null || echo "0")
        if (( 10#${requests_count:-0} > 0 )); then
            coverage=$(awk -v sum="${status_sum}" -v req="${requests_count}" 'BEGIN { printf "%.4f", sum / req }')
            if [[ "${coverage}" != "1.0000" ]]; then
                echo -e "${YELLOW}WARNING: Status coverage ${coverage} (${status_sum}/${requests_count}) - expected 1.0000${NC}" >&2
            else
                echo -e "${GREEN}Status coverage: ${coverage} (${status_sum}/${requests_count})${NC}" >&2
            fi
        fi
    fi
}

# =============================================================================
# Validate Required Percentiles (REQ-PROFILE-JSON-002)
# =============================================================================
#
# Validates that required percentile metrics (p50, p90, p99) are available
# when the benchmark has processed requests.
#
# Per REQ-PROFILE-JSON-002:
# - If total_requests > 0, percentile data MUST be available
# - If percentiles are missing, the benchmark MUST fail
# - If total_requests = 0, validation is skipped
#
# Parameters:
#   $1: p50 value (milliseconds or null)
#   $2: p90 value (milliseconds or null)
#   $3: p99 value (milliseconds or null)
#   $4: total_requests count
#
# Returns:
#   0: validation passed or skipped (no requests)
#   1: validation failed (percentiles missing when requests > 0)
# =============================================================================
validate_required_percentiles() {
    local p50="$1"
    local p90="$2"
    local p99="$3"
    local total_requests="$4"

    # Validate total_requests is numeric (防御的プログラミング)
    # Use stricter pattern to avoid leading zeros (08/09) causing arithmetic evaluation errors
    if [[ ! "${total_requests}" =~ ^(0|[1-9][0-9]*)$ ]]; then
        echo -e "${RED}ERROR: total_requests is not a valid number: ${total_requests}${NC}" >&2
        return 1
    fi

    # Skip validation if no requests
    # Use 10# prefix to force decimal interpretation
    if [[ "$((10#${total_requests}))" -eq 0 ]]; then
        return 0
    fi

    local missing=()
    # Check for missing, zero, or invalid values (per REQ-PROFILE-JSON-002)
    # Validate: non-empty, numeric, positive, non-zero
    is_valid_latency "${p50}" || missing+=("p50")
    is_valid_latency "${p90}" || missing+=("p90")
    is_valid_latency "${p99}" || missing+=("p99")

    if [[ ${#missing[@]} -gt 0 ]]; then
        echo -e "${RED}ERROR: Required percentiles missing/zero/invalid: ${missing[*]}${NC}" >&2
        echo "  Percentiles must be non-empty, numeric, positive, and non-zero." >&2
        echo "  wrk output may not contain latency distribution or values may be invalid." >&2
        echo "  Ensure wrk is configured with --latency flag." >&2
        return 1
    fi
    return 0
}

# =============================================================================
# Generate meta_extended.json (Phase Details)
# =============================================================================
#
# Generates meta_extended.json containing:
# - Rate control information (wrk2 version, target/actual RPS)
# - Integration method documentation
# - Per-phase detailed results
#
# This file is generated only when phased execution is used (MERGED_PHASE_COUNT > 1).
# The meta.json schema remains unchanged for backward compatibility.
# =============================================================================

generate_meta_extended() {
    local results_dir="$1"
    local meta_extended_file="${results_dir}/meta_extended.json"

    # Only generate if phased execution was used
    if [[ -z "${MERGED_PHASE_COUNT:-}" || "${MERGED_PHASE_COUNT}" -le 1 ]]; then
        return 0
    fi

    # Check if jq is available (required for meta_extended.json generation)
    if ! command -v jq &>/dev/null; then
        echo -e "${YELLOW}WARNING: jq is not installed. Skipping meta_extended.json generation.${NC}"
        return 0
    fi

    # Get wrk2 version
    local wrk_version
    wrk_version=$("${WRK_COMMAND:-wrk2}" -v 2>&1 | head -1 || echo "unknown")

    # Collect max target_rps from phases to determine rate_control_enabled
    # This handles cases where TARGET_RPS is not set but phases have non-zero target_rps
    # Note: target_rps is validated as non-negative integer by validate_scenario_parameters,
    # so integer-only comparison is safe here
    local max_phase_target_rps=0
    local phase_dirs_check
    phase_dirs_check=$(find "${results_dir}" -maxdepth 1 -type d -name "phase_*" 2>/dev/null | head -1)
    if [[ -n "${phase_dirs_check}" ]]; then
        for phase_dir in $(find "${results_dir}" -maxdepth 1 -type d -name "phase_*" 2>/dev/null); do
            if [[ -f "${phase_dir}/phase_result.json" ]]; then
                local phase_target
                phase_target=$(jq -r '.target_rps // 0' "${phase_dir}/phase_result.json" 2>/dev/null || echo "0")
                if [[ "${phase_target}" =~ ^[0-9]+$ ]] && [[ "${phase_target}" -gt "${max_phase_target_rps}" ]]; then
                    max_phase_target_rps="${phase_target}"
                fi
            fi
        done
    fi

    # Rate control status: enabled if TARGET_RPS is set OR any phase has non-zero target_rps
    local rate_control_enabled="false"
    local effective_target_rps="${TARGET_RPS:-0}"
    if [[ -n "${TARGET_RPS:-}" && "${TARGET_RPS}" != "0" ]]; then
        rate_control_enabled="true"
    elif [[ "${max_phase_target_rps}" -gt 0 ]]; then
        rate_control_enabled="true"
        effective_target_rps="${max_phase_target_rps}"
    fi

    # Check if RPS is within tolerance (from rps_verification.log)
    local rps_within_tolerance="null"
    if [[ -f "${results_dir}/rps_verification.log" ]]; then
        # Check if any FAIL entries exist
        if grep -q "| FAIL" "${results_dir}/rps_verification.log" 2>/dev/null; then
            rps_within_tolerance="false"
        else
            rps_within_tolerance="true"
        fi
    fi

    # Collect phase information
    local phases_json="[]"
    local phase_dirs
    phase_dirs=$(find "${results_dir}" -maxdepth 1 -type d -name "phase_*" 2>/dev/null | sort)

    for phase_dir in ${phase_dirs}; do
        if [[ -d "${phase_dir}" && -f "${phase_dir}/phase_result.json" ]]; then
            local phase_json target_rps actual_rps duration_seconds

            # Read phase result
            target_rps=$(jq -r '.target_rps // 0' "${phase_dir}/phase_result.json" 2>/dev/null || echo "0")
            actual_rps=$(jq -r '.actual_rps // 0' "${phase_dir}/phase_result.json" 2>/dev/null || echo "0")
            duration_seconds=$(jq -r '.duration_seconds // 0' "${phase_dir}/phase_result.json" 2>/dev/null || echo "0")

            # Calculate deviation percent (avoid division by zero or non-numeric)
            local deviation_percent="0"
            if [[ "${target_rps}" =~ ^[0-9]+(\.[0-9]+)?$ ]] && awk -v t="${target_rps}" 'BEGIN { exit (t > 0) ? 0 : 1 }'; then
                deviation_percent=$(awk -v t="${target_rps}" -v a="${actual_rps}" \
                    'BEGIN { printf "%.2f", ((a - t) / t) * 100 }')
            fi

            # Build phase JSON with deviation_percent
            phase_json=$(jq -c --argjson dev "${deviation_percent}" \
                '. + {deviation_percent: $dev}' "${phase_dir}/phase_result.json" 2>/dev/null)

            # Append to phases array
            if [[ -n "${phase_json}" && "${phase_json}" != "null" ]]; then
                phases_json=$(echo "${phases_json}" | jq -c --argjson p "${phase_json}" '. + [$p]')
            fi
        fi
    done

    # Prepare target_rps for JSON (use effective_target_rps if non-zero, else null)
    local target_rps_json="null"
    if [[ "${effective_target_rps}" -gt 0 ]]; then
        target_rps_json="${effective_target_rps}"
    fi

    # Prepare actual_rps for JSON
    local actual_rps_json="null"
    if [[ -n "${MERGED_RPS:-}" ]]; then
        actual_rps_json="${MERGED_RPS}"
    fi

    # Generate meta_extended.json
    jq -n \
        --arg version "1.0" \
        --arg wrk_version "${wrk_version}" \
        --argjson rate_control_enabled "${rate_control_enabled}" \
        --argjson target_rps "${target_rps_json}" \
        --argjson actual_rps "${actual_rps_json}" \
        --argjson rps_within_tolerance "${rps_within_tolerance}" \
        --arg rps_method "duration_weighted_average" \
        --arg latency_p99_method "max" \
        --arg error_rate_method "requests_weighted_average" \
        --argjson phases "${phases_json}" \
        '{
            version: $version,
            rate_control: {
                wrk_version: $wrk_version,
                rate_control_enabled: $rate_control_enabled,
                target_rps: $target_rps,
                actual_rps: $actual_rps,
                rps_within_tolerance: $rps_within_tolerance
            },
            integration: {
                rps_method: $rps_method,
                latency_p99_method: $latency_p99_method,
                error_rate_method: $error_rate_method
            },
            phases: $phases
        }' > "${meta_extended_file}"

    echo -e "${GREEN}meta_extended.json generated (v1.0)${NC}"
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
        # Determine if sudo is required first
        local use_sudo=false
        if ! perf record -F 99 -p "${api_pid}" -g -o "${PERF_DATA_FILE}" -- sleep 0.5 2>/dev/null; then
            if sudo -n true 2>/dev/null; then
                use_sudo=true
            else
                echo -e "${YELLOW}Warning: Cannot run perf (permission denied). Skipping profiling.${NC}"
                PROFILE_MODE=false
                return 0
            fi
        fi
        rm -f "${PERF_DATA_FILE}" 2>/dev/null

        # Try with --call-graph dwarf first, fallback to -g (fp) if unsupported
        # Use larger stack size (16KB) for dwarf to handle deep call stacks
        local callgraph_method="--call-graph dwarf,16384"
        local perf_cmd="perf record"
        if [[ "${use_sudo}" == "true" ]]; then
            perf_cmd="sudo perf record"
        fi

        if ! ${perf_cmd} -F 99 -p "${api_pid}" ${callgraph_method} -o "${PERF_DATA_FILE}" -- sleep 0.5 2>/dev/null; then
            echo -e "${YELLOW}Warning: --call-graph dwarf not supported, falling back to -g (fp)${NC}"
            callgraph_method="-g"
            # Re-validate with -g fallback
            if ! ${perf_cmd} -F 99 -p "${api_pid}" ${callgraph_method} -o "${PERF_DATA_FILE}" -- sleep 0.5 2>/dev/null; then
                echo -e "${YELLOW}Warning: perf -g also failed. Skipping profiling.${NC}"
                PROFILE_MODE=false
                rm -f "${PERF_DATA_FILE}" 2>/dev/null
                return 0
            fi
        fi
        rm -f "${PERF_DATA_FILE}" 2>/dev/null

        # Execute actual recording
        ${perf_cmd} -F 99 -p "${api_pid}" ${callgraph_method} -o "${PERF_DATA_FILE}" &
        export PERF_RECORD_PID=$!
        if [[ "${use_sudo}" == "true" ]]; then
            export PERF_NEEDS_SUDO=true
        fi
        echo "  perf recording started (PID: ${api_pid}, method: ${callgraph_method}, sudo: ${use_sudo})"
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

# =============================================================================
# Log Resolved Parameters
# =============================================================================
#
# Outputs the resolved parameters from scenario YAML to a log file for debugging.
# This helps verify that scenario configuration is correctly applied.
# =============================================================================

log_resolved_params() {
    local script_name="$1"
    local rate_option="$2"
    local log_file="${RESULTS_DIR}/resolved_params.log"
    local timestamp
    timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    # Generate log file using printf instead of heredoc (REQ-PROFILE-JSON-001 compliance)
    {
        printf "# Resolved Parameters from Scenario YAML\n"
        printf "# Generated at: %s\n\n" "${timestamp}"
        printf "## Source\n"
        printf "scenario_file: %s\n\n" "${SCENARIO_FILE}"
        printf "## Resolved Values\n"
        printf "TARGET_RPS=%s\n" "${TARGET_RPS:-null}"
        printf "RPS_PROFILE=%s\n" "${RPS_PROFILE:-steady}"
        printf "LOAD_PROFILE=%s\n" "${LOAD_PROFILE:-steady}"
        printf "DURATION=%s\n" "${DURATION}"
        printf "THREADS=%s\n" "${THREADS}"
        printf "CONNECTIONS=%s\n\n" "${CONNECTIONS}"
        printf "## Cache Configuration\n"
        printf "CACHE_MODE=%s\n" "${CACHE_MODE:-none}"
        printf "CACHE_ENABLED=%s\n" "${CACHE_ENABLED:-true}"
        printf "CACHE_STRATEGY=%s\n" "${CACHE_STRATEGY:-read-through}"
        printf "CACHE_TTL_SECS=%s\n" "${CACHE_TTL_SECS:-60}"
        printf "HIT_RATE=%s\n\n" "${HIT_RATE:-50}"
        printf "## Storage Configuration\n"
        printf "STORAGE_MODE=%s\n" "${STORAGE_MODE:-in_memory}"
        printf "DATA_SCALE=%s\n\n" "${DATA_SCALE:-1e4}"
        printf "## Concurrency\n"
        printf "WORKER_THREADS=%s\n" "${WORKER_THREADS:-4}"
        printf "DATABASE_POOL_SIZE=%s\n" "${DATABASE_POOL_SIZE:-16}"
        printf "REDIS_POOL_SIZE=%s\n" "${REDIS_POOL_SIZE:-8}"
        printf "POOL_SIZES=%s\n\n" "${POOL_SIZES:-24}"
        printf "## Error Configuration\n"
        printf "FAIL_RATE=%s\n" "${FAIL_RATE:-0}"
        printf "RETRY=%s\n" "${RETRY:-false}"
        printf "RETRY_COUNT=%s\n" "${RETRY_COUNT:-0}"
        printf "ID_POOL_SIZE=%s\n\n" "${ID_POOL_SIZE:-10}"
        printf "## Multi-Phase Load Profile Parameters\n"
        printf "MIN_RPS=%s\n" "${MIN_RPS:-10}"
        printf "STEP_COUNT=%s\n" "${STEP_COUNT:-4}"
        printf "RAMP_UP_SECONDS=%s\n" "${RAMP_UP_SECONDS:-10}"
        printf "RAMP_DOWN_SECONDS=%s\n" "${RAMP_DOWN_SECONDS:-10}"
        printf "BURST_INTERVAL_SECONDS=%s\n" "${BURST_INTERVAL_SECONDS:-20}"
        printf "BURST_DURATION_SECONDS=%s\n" "${BURST_DURATION_SECONDS:-5}"
        printf "BURST_MULTIPLIER=%s\n\n" "${BURST_MULTIPLIER:-3}"
        printf "## wrk2 Execution Command\n"
        printf "%s -t%s -c%s -d%s %s --latency --script=scripts/%s.lua %s\n" \
            "${WRK_COMMAND}" "${THREADS}" "${CONNECTIONS}" "${DURATION}" "${rate_option}" "${script_name}" "${API_URL}"
    } > "${log_file}"

    echo "  Resolved parameters logged to: ${log_file}"
}

# =============================================================================
# Phased Benchmark Execution
# =============================================================================
#
# Implements multi-phase benchmark execution for load profiles:
# - steady (or constant): Single phase at TARGET_RPS for DURATION_SECONDS
# - step_up: N steps with progressively increasing RPS
# - ramp_up_down: Ramp up -> Sustain -> Ramp down phases
# - burst: Alternating burst/normal cycles
#
# Each phase runs wrk2 separately, results are merged for meta.json generation.
# =============================================================================

# Verify that actual RPS is within acceptable tolerance of target RPS
# @param $1 target_rps - Target RPS configured for this phase
# @param $2 actual_rps - Actual RPS achieved
# @param $3 phase_name - Name of the phase (for logging)
# @return 0 if within tolerance or SKIP, 1 if FAIL in strict mode
verify_rps_accuracy() {
    local target_rps="$1"
    local actual_rps="$2"
    local phase_name="${3:-main}"
    local tolerance_percent="5"
    local tolerance_absolute="5"  # 絶対誤差 ±5 RPS（target < 10 用）
    local tolerance_mode="${RPS_TOLERANCE_MODE:-strict}"

    # Validate RESULTS_DIR is set and non-empty
    if [[ -z "${RESULTS_DIR:-}" ]]; then
        echo -e "${RED}ERROR: RESULTS_DIR is not set. Cannot write RPS verification log.${NC}"
        return 1
    fi

    local log_file="${RESULTS_DIR}/rps_verification.log"

    # Validate tolerance_mode: only "strict" or "warn" are valid, default to "strict" for unknown values
    if [[ "${tolerance_mode}" != "strict" && "${tolerance_mode}" != "warn" ]]; then
        echo -e "${YELLOW}WARNING: Unknown RPS_TOLERANCE_MODE '${tolerance_mode}', defaulting to 'strict'${NC}"
        tolerance_mode="strict"
    fi

    # Validate actual_rps is a valid number (integer or decimal)
    # actual_rps is extracted from wrk2 output and may be empty or non-numeric on parse failure
    if [[ -z "${actual_rps}" ]] || ! echo "${actual_rps}" | grep -qE '^[0-9]+(\.[0-9]+)?$'; then
        echo -e "${YELLOW}WARNING: Invalid actual_rps '${actual_rps}', treating as 0${NC}"
        actual_rps="0"
    fi

    # Ensure RESULTS_DIR exists before creating log file
    mkdir -p "${RESULTS_DIR}"

    if [[ ! -f "${log_file}" ]]; then
        echo "# RPS Verification Log" > "${log_file}"
        echo "# Tolerance: ±${tolerance_percent}% (relative) or ±${tolerance_absolute} RPS (absolute for target<10)" >> "${log_file}"
        echo "# Mode: ${tolerance_mode}" >> "${log_file}"
        echo "" >> "${log_file}"
    fi

    local timestamp
    timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    # Step 1: target_rps == 0 or empty → SKIP（乖離率計算対象外）
    # Note: target_rps is validated as non-negative integer by validate_scenario_parameters
    if [[ -z "${target_rps}" || "${target_rps}" == "0" ]]; then
        echo "${timestamp} | ${phase_name} | target=0 | actual=${actual_rps} | SKIP (no target_rps)" | tee -a "${log_file}"
        return 0
    fi

    # Step 2: target_rps < 10 → 絶対誤差 ±5 RPS
    # Note: target_rps is validated as non-negative integer by validate_scenario_parameters
    # so -lt comparison is safe here
    if [[ "${target_rps}" -lt 10 ]]; then
        echo -e "${YELLOW}WARNING: target_rps (${target_rps}) < 10, using absolute tolerance ±${tolerance_absolute} RPS${NC}"
        local abs_diff
        abs_diff=$(awk -v t="${target_rps}" -v a="${actual_rps}" 'BEGIN { diff = a - t; print (diff < 0 ? -diff : diff) }')
        local result_line="${timestamp} | ${phase_name} | target=${target_rps} | actual=${actual_rps} | abs_diff=${abs_diff}"

        if (( $(echo "${abs_diff} <= ${tolerance_absolute}" | bc -l) )); then
            echo "${result_line} | PASS (absolute ±${tolerance_absolute})" | tee -a "${log_file}"
            return 0
        else
            echo "${result_line} | FAIL (exceeds ±${tolerance_absolute} RPS)" | tee -a "${log_file}"
            if [[ "${tolerance_mode}" == "strict" ]]; then
                echo -e "${RED}ERROR: RPS verification failed. Set RPS_TOLERANCE_MODE=warn to continue.${NC}"
                return 1
            else
                echo -e "${YELLOW}WARNING: RPS verification failed but continuing (tolerance_mode=warn)${NC}"
                return 0
            fi
        fi
    fi

    # Step 3: target_rps >= 10 → 相対誤差 ±5%
    # Calculate deviation with fixed precision (%.6f) for reliable bc comparison
    local abs_deviation
    abs_deviation=$(awk -v t="${target_rps}" -v a="${actual_rps}" \
        'BEGIN { diff = ((a - t) / t) * 100; printf "%.6f", (diff < 0 ? -diff : diff) }')
    local deviation_display
    deviation_display=$(awk -v t="${target_rps}" -v a="${actual_rps}" \
        'BEGIN { printf "%.2f", ((a - t) / t) * 100 }')

    local result_line="${timestamp} | ${phase_name} | target=${target_rps} | actual=${actual_rps} | deviation=${deviation_display}%"

    if (( $(echo "${abs_deviation} <= ${tolerance_percent}" | bc -l) )); then
        echo "${result_line} | PASS" | tee -a "${log_file}"
        return 0
    else
        echo "${result_line} | FAIL (exceeds ±${tolerance_percent}%)" | tee -a "${log_file}"
        if [[ "${tolerance_mode}" == "strict" ]]; then
            echo -e "${RED}ERROR: RPS verification failed. Set RPS_TOLERANCE_MODE=warn to continue.${NC}"
            return 1
        else
            echo -e "${YELLOW}WARNING: RPS verification failed but continuing (tolerance_mode=warn)${NC}"
            return 0
        fi
    fi
}

# Run a single phase of the benchmark
# @param $1 script_name - Lua script name (without .lua extension)
# @param $2 phase_dir - Directory to store phase results
# @param $3 target_rps - Target RPS for this phase
# @param $4 duration - Duration in seconds for this phase
# @param $5 phase_name - Human-readable phase name for logging
run_single_phase() {
    local script_name="$1"
    local phase_dir="$2"
    local target_rps="$3"
    local duration="$4"
    local phase_name="$5"

    # Default RPS if not specified (wrk2 requires -R option)
    local DEFAULT_RPS=100
    if [[ -z "${target_rps}" || "${target_rps}" == "0" ]]; then
        echo -e "${YELLOW}WARNING: target_rps is 0 or not specified. Using default: ${DEFAULT_RPS} RPS${NC}"
        target_rps="${DEFAULT_RPS}"
    fi

    mkdir -p "${phase_dir}"
    local result_file="${phase_dir}/wrk.txt"
    local phase_log="${phase_dir}/phase.log"

    echo "[${phase_name}] Starting: target_rps=${target_rps}, duration=${duration}s" | tee "${phase_log}"

    # wrk2 always requires -R option
    local rate_option="-R${target_rps}"

    # Set LUA_RESULTS_DIR for Lua scripts to output lua_metrics.json
    export LUA_RESULTS_DIR="${phase_dir}"

    cd "${SCRIPT_DIR}"
    ${WRK_COMMAND} -t"${THREADS}" -c"${CONNECTIONS}" -d"${duration}s" \
        ${rate_option} \
        --latency \
        --script="scripts/${script_name}.lua" \
        "${API_URL}" 2>&1 | tee "${phase_dir}/raw_wrk.txt" | tee "${result_file}"

    local actual_rps
    actual_rps=$(grep "Requests/sec:" "${result_file}" 2>/dev/null | awk '{print $2}' || echo "0")

    # Validate actual_rps is a valid number for JSON output
    if [[ -z "${actual_rps}" ]] || ! echo "${actual_rps}" | grep -qE '^[0-9]+(\.[0-9]+)?$'; then
        echo -e "${YELLOW}WARNING: Invalid actual_rps '${actual_rps}' from wrk2 output, treating as 0${NC}"
        actual_rps="0"
    fi

    echo "[${phase_name}] Completed: actual_rps=${actual_rps}" | tee -a "${phase_log}"

    # Save phase result as JSON for merge_phase_results
    # Use jq to ensure safe JSON generation (REQ-PROFILE-JSON-001)
    jq -n \
        --arg phase "${phase_name}" \
        --argjson target_rps "${target_rps}" \
        --argjson actual_rps "${actual_rps}" \
        --argjson duration_seconds "${duration}" \
        '{
            "phase": $phase,
            "target_rps": $target_rps,
            "actual_rps": $actual_rps,
            "duration_seconds": $duration_seconds
        }' > "${phase_dir}/phase_result.json"

    # Verify RPS accuracy
    if ! verify_rps_accuracy "${target_rps}" "${actual_rps}" "${phase_name}"; then
        echo -e "${RED}Phase ${phase_name} failed RPS verification${NC}"
        # In strict mode, propagate failure
        return 1
    fi
}

# Merge results from all phases into a unified wrk.txt for meta.json generation
# @param $1 results_base_dir - Base directory containing phase_* subdirectories
merge_phase_results() {
    local results_base_dir="$1"
    local merged_wrk="${results_base_dir}/wrk.txt"

    # Collect all phase directories
    local phase_dirs
    phase_dirs=$(find "${results_base_dir}" -maxdepth 1 -type d -name "phase_*" | sort)

    if [[ -z "${phase_dirs}" ]]; then
        echo -e "${YELLOW}WARNING: No phase directories found to merge${NC}"
        return 1
    fi

    local phase_count=0
    local total_requests=0
    local weighted_rps_sum=0
    local total_duration=0
    local max_p99=0
    local total_http_errors=0
    local total_socket_errors=0

    # Initialize latency tracking arrays
    declare -a avg_latencies=()
    declare -a p50_latencies=()
    declare -a p75_latencies=()
    declare -a p90_latencies=()
    declare -a p95_latencies=()
    declare -a p99_latencies=()

    for phase_dir in ${phase_dirs}; do
        if [[ -d "${phase_dir}" ]]; then
            local wrk_file="${phase_dir}/wrk.txt"
            local phase_json="${phase_dir}/phase_result.json"

            if [[ -f "${wrk_file}" && -f "${phase_json}" ]]; then
                phase_count=$((phase_count + 1))

                # Extract metrics from wrk output
                local requests
                requests=$(grep -m1 "requests in" "${wrk_file}" 2>/dev/null | awk '{print $1}' || echo "0")
                [[ ! "${requests}" =~ ^[0-9]+$ ]] && requests=0

                local rps duration
                rps=$(jq -r '.actual_rps // 0' "${phase_json}" 2>/dev/null || echo "0")
                duration=$(jq -r '.duration_seconds // 0' "${phase_json}" 2>/dev/null || echo "0")

                # Socket errors (tracked separately, NOT included in error_rate)
                local socket_errors=0
                if grep -q "Socket errors:" "${wrk_file}" 2>/dev/null; then
                    local connect_err read_err write_err timeout_err
                    connect_err=$(grep "Socket errors:" "${wrk_file}" | sed 's/.*connect \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
                    read_err=$(grep "Socket errors:" "${wrk_file}" | sed 's/.*read \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
                    write_err=$(grep "Socket errors:" "${wrk_file}" | sed 's/.*write \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
                    timeout_err=$(grep "Socket errors:" "${wrk_file}" | sed 's/.*timeout \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
                    socket_errors=$((connect_err + read_err + write_err + timeout_err))
                fi
                total_socket_errors=$((total_socket_errors + socket_errors))

                # HTTP errors (used for error_rate calculation)
                local http_errors=0
                if grep -q "Non-2xx or 3xx responses:" "${wrk_file}" 2>/dev/null; then
                    http_errors=$(grep -m1 "Non-2xx or 3xx responses:" "${wrk_file}" | awk '{print $NF}' 2>/dev/null || echo "0")
                    [[ ! "${http_errors}" =~ ^[0-9]+$ ]] && http_errors=0
                fi
                total_http_errors=$((total_http_errors + http_errors))

                total_requests=$((total_requests + requests))
                weighted_rps_sum=$(echo "${weighted_rps_sum} + (${rps} * ${duration})" | bc 2>/dev/null || echo "${weighted_rps_sum}")
                total_duration=$((total_duration + duration))

                # Extract p99 latency and track maximum
                local p99_raw
                p99_raw=$(grep -E "^[[:space:]]+99[.0-9]*%" "${wrk_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "")
                if [[ -n "${p99_raw}" ]]; then
                    local p99_ms
                    p99_ms=$(parse_latency_to_ms "${p99_raw}")
                    if [[ -n "${p99_ms}" ]]; then
                        local compare_result
                        compare_result=$(echo "${p99_ms} > ${max_p99}" | bc -l 2>/dev/null || echo "0")
                        if [[ "${compare_result}" == "1" ]]; then
                            max_p99="${p99_ms}"
                        fi
                    fi
                fi

                # Collect latencies for potential averaging
                local avg_lat p50_lat p75_lat p90_lat p95_lat p99_lat
                avg_lat=$(grep "Latency" "${wrk_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "")
                p50_lat=$(grep -E "^[[:space:]]+50[.0-9]*%" "${wrk_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "")
                p75_lat=$(grep -E "^[[:space:]]+75[.0-9]*%" "${wrk_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "")
                p90_lat=$(grep -E "^[[:space:]]+90[.0-9]*%" "${wrk_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "")
                p95_lat=$(grep -E "^[[:space:]]+95[.0-9]*%" "${wrk_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "")
                p99_lat=$(grep -E "^[[:space:]]+99[.0-9]*%" "${wrk_file}" 2>/dev/null | head -1 | awk '{print $2}' || echo "")

                [[ -n "${avg_lat}" ]] && avg_latencies+=("${avg_lat}")
                [[ -n "${p50_lat}" ]] && p50_latencies+=("${p50_lat}")
                [[ -n "${p75_lat}" ]] && p75_latencies+=("${p75_lat}")
                [[ -n "${p90_lat}" ]] && p90_latencies+=("${p90_lat}")
                [[ -n "${p95_lat}" ]] && p95_latencies+=("${p95_lat}")
                [[ -n "${p99_lat}" ]] && p99_latencies+=("${p99_lat}")
            fi
        fi
    done

    if [[ "${total_duration}" -eq 0 ]]; then
        echo -e "${YELLOW}WARNING: Total duration is 0, cannot calculate averages${NC}"
        total_duration=1
    fi

    # Calculate weighted average RPS
    local avg_rps
    avg_rps=$(echo "scale=2; ${weighted_rps_sum} / ${total_duration}" | bc 2>/dev/null || echo "0")

    # Error rate will be read from merged lua_metrics.json instead of wrk output
    # This ensures consistency with http_status counts (REQ-PIPELINE-002)
    # Fallback to wrk-based calculation only if lua_metrics is unavailable
    local error_rate=""

    # Use the last phase's latency values for the merged output (representative of peak load)
    # Exception: p99 uses max_p99 (worst case across all phases) for conservative reporting
    local last_avg="" last_p50="" last_p75="" last_p90="" last_p95="" last_p99=""
    if [[ ${#avg_latencies[@]} -gt 0 ]]; then
        last_avg="${avg_latencies[$((${#avg_latencies[@]} - 1))]}"
    fi
    if [[ ${#p50_latencies[@]} -gt 0 ]]; then
        last_p50="${p50_latencies[$((${#p50_latencies[@]} - 1))]}"
    fi
    if [[ ${#p75_latencies[@]} -gt 0 ]]; then
        last_p75="${p75_latencies[$((${#p75_latencies[@]} - 1))]}"
    fi
    if [[ ${#p90_latencies[@]} -gt 0 ]]; then
        last_p90="${p90_latencies[$((${#p90_latencies[@]} - 1))]}"
    fi
    if [[ ${#p95_latencies[@]} -gt 0 ]]; then
        last_p95="${p95_latencies[$((${#p95_latencies[@]} - 1))]}"
    fi
    if [[ ${#p99_latencies[@]} -gt 0 ]]; then
        last_p99="${p99_latencies[$((${#p99_latencies[@]} - 1))]}"
    fi

    # Format max_p99 for display (convert from ms number to human-readable string)
    local max_p99_display="N/A"
    if [[ -n "${max_p99}" ]]; then
        local max_p99_compare
        max_p99_compare=$(echo "${max_p99} > 0" | bc -l 2>/dev/null || echo "0")
        if [[ "${max_p99_compare}" == "1" ]]; then
            # Format: if >= 1000ms show as Xs, if >= 1ms show as Xms, else show as Xus
            if awk -v val="${max_p99}" 'BEGIN { exit (val >= 1000) ? 0 : 1 }'; then
                max_p99_display=$(awk -v val="${max_p99}" 'BEGIN { printf "%.2fs", val / 1000 }')
            elif awk -v val="${max_p99}" 'BEGIN { exit (val >= 1) ? 0 : 1 }'; then
                max_p99_display=$(awk -v val="${max_p99}" 'BEGIN { printf "%.2fms", val }')
            else
                max_p99_display=$(awk -v val="${max_p99}" 'BEGIN { printf "%.2fus", val * 1000 }')
            fi
        fi
    fi

    # Generate merged wrk.txt in wrk-compatible format
    # Generate merged wrk output using printf instead of heredoc (REQ-PROFILE-JSON-001 compliance)
    {
        printf "Running %ss test @ %s\n" "${total_duration}" "${API_URL}"
        printf "  %s threads and %s connections\n\n" "${THREADS}" "${CONNECTIONS}"
        printf -- "=== Merged Results (%s profile, %s phases) ===\n\n" "${RPS_PROFILE}" "${phase_count}"
        printf "  Thread Stats   Avg      Stdev     Max   +/- Stdev\n"
        printf "    Latency   %s\n\n" "${last_avg:-N/A}"
        printf "  Latency Distribution\n"
        printf "     50%%    %s\n" "${last_p50:-N/A}"
        printf "     75%%    %s\n" "${last_p75:-N/A}"
        printf "     90%%    %s\n" "${last_p90:-N/A}"
        printf "     99%%    %s\n" "${max_p99_display}"
        printf "     99.9%%  N/A\n"
        printf "  Max P99 (across phases): %s\n" "${max_p99_display}"
        printf "  %s requests in %ss\n" "${total_requests}" "${total_duration}"
        printf "Requests/sec: %s\n" "${avg_rps}"
        printf "Transfer/sec: N/A (merged result)\n\n"
        printf -- "--- Phase Details ---\n"
    } > "${merged_wrk}"

    # Append phase summaries
    for phase_dir in ${phase_dirs}; do
        if [[ -d "${phase_dir}" && -f "${phase_dir}/phase_result.json" ]]; then
            local phase_name target actual dur
            phase_name=$(jq -r '.phase // "unknown"' "${phase_dir}/phase_result.json" 2>/dev/null)
            target=$(jq -r '.target_rps // 0' "${phase_dir}/phase_result.json" 2>/dev/null)
            actual=$(jq -r '.actual_rps // 0' "${phase_dir}/phase_result.json" 2>/dev/null)
            dur=$(jq -r '.duration_seconds // 0' "${phase_dir}/phase_result.json" 2>/dev/null)
            echo "  ${phase_name}: target=${target} RPS, actual=${actual} RPS, duration=${dur}s" >> "${merged_wrk}"
        fi
    done

    # Merge lua_metrics.json from all phases BEFORE exporting metrics
    local lua_metrics_files=()
    for phase_dir in ${phase_dirs}; do
        local lua_metrics="${phase_dir}/lua_metrics.json"
        if [[ -f "${lua_metrics}" ]]; then
            lua_metrics_files+=("${lua_metrics}")
        fi
    done

    if [[ ${#lua_metrics_files[@]} -gt 0 ]]; then
        echo -e "${CYAN}Merging ${#lua_metrics_files[@]} lua_metrics.json files...${NC}"
        python3 "${SCRIPT_DIR}/scripts/merge_lua_metrics.py" \
            --output "${results_base_dir}/lua_metrics.json" \
            "${lua_metrics_files[@]}"
    else
        echo -e "${YELLOW}WARNING: No lua_metrics.json files found to merge${NC}"
    fi

    # Read http_4xx, http_5xx from merged lua_metrics.json
    local merged_http_4xx=0 merged_http_5xx=0
    if [[ -f "${results_base_dir}/lua_metrics.json" ]] && command -v jq &> /dev/null; then
        merged_http_4xx=$(jq -r '.http_4xx // 0' "${results_base_dir}/lua_metrics.json" 2>/dev/null)
        merged_http_5xx=$(jq -r '.http_5xx // 0' "${results_base_dir}/lua_metrics.json" 2>/dev/null)
        [[ ! "${merged_http_4xx}" =~ ^[0-9]+$ ]] && merged_http_4xx=0
        [[ ! "${merged_http_5xx}" =~ ^[0-9]+$ ]] && merged_http_5xx=0
    fi

    # Compute error_rate using single-source formula (REQ-MET-P3-002)
    error_rate=$(compute_error_rate "${total_requests}" "${merged_http_4xx}" "${merged_http_5xx}" "${total_socket_errors:-0}")
    echo -e "${CYAN}Phase merge: computed error_rate: ${error_rate}${NC}"

    # Export merged values as environment variables for meta.json generation
    # IMPORTANT: error_rate must be read from lua_metrics.json BEFORE export
    export MERGED_RPS="${avg_rps}"
    export MERGED_REQUESTS="${total_requests}"
    export MERGED_DURATION="${total_duration}"
    # Export latencies in milliseconds (parse_latency_to_ms applied)
    if [[ -n "${last_p50}" ]]; then
        export MERGED_P50="$(parse_latency_to_ms "${last_p50}")"
    fi
    if [[ -n "${last_p90}" ]]; then
        export MERGED_P90="$(parse_latency_to_ms "${last_p90}")"
    fi
    if [[ -n "${last_p95}" ]]; then
        export MERGED_P95="$(parse_latency_to_ms "${last_p95}")"
    fi
    export MERGED_P99="${max_p99}"
    export MERGED_ERROR_RATE="${error_rate}"
    export MERGED_PHASE_COUNT="${phase_count}"
    export MERGED_SOCKET_ERRORS="${total_socket_errors}"

    # Concatenate raw_wrk.txt files
    local raw_wrk_files=()
    for phase_dir in ${phase_dirs}; do
        local raw_wrk="${phase_dir}/raw_wrk.txt"
        if [[ -f "${raw_wrk}" ]]; then
            raw_wrk_files+=("${raw_wrk}")
        fi
    done

    if [[ ${#raw_wrk_files[@]} -gt 0 ]]; then
        cat "${raw_wrk_files[@]}" > "${results_base_dir}/raw_wrk_all.txt"
        echo -e "${CYAN}Concatenated ${#raw_wrk_files[@]} raw_wrk.txt files${NC}"
    fi

    echo -e "${GREEN}Merged ${phase_count} phase results${NC}"
}

# Run phased benchmark based on RPS_PROFILE
# @param $1 script_name - Lua script name
# @param $2 results_base_dir - Base directory for results
run_phased_benchmark() {
    local script_name="$1"
    local results_base_dir="$2"
    local profile="${RPS_PROFILE:-steady}"

    echo ""
    echo "Running phased benchmark: profile=${profile}"
    echo ""

    # Extract DURATION_SECONDS from DURATION (remove 's' suffix)
    local duration_seconds
    duration_seconds=$(echo "${DURATION}" | sed 's/s$//')
    if [[ -z "${duration_seconds}" || ! "${duration_seconds}" =~ ^[0-9]+$ ]]; then
        duration_seconds=30
    fi

    case "${profile}" in
        steady|constant)
            # Single phase at steady/constant RPS
            run_single_phase "${script_name}" "${results_base_dir}/phase_main" \
                "${TARGET_RPS:-0}" "${duration_seconds}" "main"
            merge_phase_results "${results_base_dir}"
            ;;

        step_up)
            # Gradual steps up to TARGET_RPS
            local step_count="${STEP_COUNT:-4}"
            local base_step_duration=$((duration_seconds / step_count))
            local min_rps="${MIN_RPS:-10}"
            local target_rps="${TARGET_RPS:-100}"
            local rps_range=$((target_rps - min_rps))

            # Calculate remainder to distribute to final step
            local total_allocated_duration=0

            echo "Step-up configuration: ${step_count} steps"
            echo "  RPS range: ${min_rps} -> ${target_rps}"

            for i in $(seq 1 "${step_count}"); do
                local step_rps step_duration
                # Final step guarantees TARGET_RPS (avoid rounding issues)
                if [[ "${i}" -eq "${step_count}" ]]; then
                    step_rps="${target_rps}"
                    # Final step uses remaining time to avoid truncation
                    step_duration=$((duration_seconds - total_allocated_duration))
                else
                    # step_rps = min_rps + (rps_range / step_count) * i
                    step_rps=$((min_rps + (rps_range * i) / step_count))
                    step_duration="${base_step_duration}"
                fi
                total_allocated_duration=$((total_allocated_duration + step_duration))

                echo ""
                echo "--- Step ${i}/${step_count}: ${step_rps} RPS, ${step_duration}s ---"
                run_single_phase "${script_name}" "${results_base_dir}/phase_step_${i}" \
                    "${step_rps}" "${step_duration}" "step_${i}"
            done

            merge_phase_results "${results_base_dir}"
            ;;

        ramp_up_down)
            # Linear ramp up, sustain at peak, linear ramp down
            local ramp_up="${RAMP_UP_SECONDS:-10}"
            local ramp_down="${RAMP_DOWN_SECONDS:-10}"
            local total_ramp=$((ramp_up + ramp_down))
            local min_rps="${MIN_RPS:-10}"
            local target_rps="${TARGET_RPS:-100}"

            # Boundary condition: ramp_up + ramp_down > duration
            # Scale down proportionally to fit within duration
            if [[ "${total_ramp}" -gt "${duration_seconds}" ]]; then
                echo -e "${YELLOW}WARNING: ramp_up + ramp_down (${total_ramp}s) > duration (${duration_seconds}s). Scaling down.${NC}"
                # Proportionally scale ramp_up and ramp_down to fit within duration
                local original_ramp_up="${ramp_up}"
                local original_ramp_down="${ramp_down}"
                ramp_up=$((duration_seconds * original_ramp_up / total_ramp))
                ramp_down=$((duration_seconds - ramp_up))
                echo "  Adjusted: ramp_up=${ramp_up}s, ramp_down=${ramp_down}s"
            fi

            local sustain_duration=$((duration_seconds - ramp_up - ramp_down))

            echo "Ramp-up-down configuration:"
            echo "  Ramp up: ${ramp_up}s (${min_rps} -> ${target_rps} RPS)"
            echo "  Sustain: ${sustain_duration}s at ${target_rps} RPS"
            echo "  Ramp down: ${ramp_down}s (${target_rps} -> ${min_rps} RPS)"

            # Boundary condition: sustain duration is negative or zero
            if [[ "${sustain_duration}" -le 0 ]]; then
                echo -e "${YELLOW}WARNING: sustain duration is ${sustain_duration}s, skipping sustain phase${NC}"
                sustain_duration=0
            fi

            # Phase 1: Ramp up (skip if duration is 0)
            # Note: wrk2 uses constant rate, so we target the peak RPS at end of ramp
            if [[ "${ramp_up}" -gt 0 ]]; then
                echo ""
                echo "--- Ramp Up Phase (${ramp_up}s) ---"
                run_single_phase "${script_name}" "${results_base_dir}/phase_ramp_up" \
                    "${target_rps}" "${ramp_up}" "ramp_up"
            else
                echo ""
                echo "--- Ramp Up Phase: skipped (0s) ---"
            fi

            # Phase 2: Sustain (skip if duration <= 0)
            if [[ "${sustain_duration}" -gt 0 ]]; then
                echo ""
                echo "--- Sustain Phase (${sustain_duration}s) ---"
                run_single_phase "${script_name}" "${results_base_dir}/phase_sustain" \
                    "${target_rps}" "${sustain_duration}" "sustain"
            else
                echo ""
                echo "--- Sustain Phase: skipped (0s) ---"
            fi

            # Phase 3: Ramp down (skip if duration is 0)
            if [[ "${ramp_down}" -gt 0 ]]; then
                echo ""
                echo "--- Ramp Down Phase (${ramp_down}s) ---"
                run_single_phase "${script_name}" "${results_base_dir}/phase_ramp_down" \
                    "${min_rps}" "${ramp_down}" "ramp_down"
            else
                echo ""
                echo "--- Ramp Down Phase: skipped (0s) ---"
            fi

            merge_phase_results "${results_base_dir}"
            ;;

        burst)
            # Periodic bursts (spikes)
            local burst_interval="${BURST_INTERVAL_SECONDS:-20}"
            local burst_duration="${BURST_DURATION_SECONDS:-5}"
            local burst_multiplier="${BURST_MULTIPLIER:-3}"
            local min_rps="${MIN_RPS:-10}"
            local target_rps="${TARGET_RPS:-100}"

            # Validate burst_multiplier > 0
            local is_positive
            is_positive=$(echo "${burst_multiplier}" | awk '{ print ($1 > 0) ? "yes" : "no" }')
            if [[ "${is_positive}" != "yes" ]]; then
                echo -e "${RED}ERROR: burst_multiplier must be > 0 (got: ${burst_multiplier})${NC}"
                return 1
            fi

            # Calculate base RPS (normal phase): target_rps / burst_multiplier
            # Use bc for floating point division
            local base_rps
            base_rps=$(echo "scale=0; ${target_rps} / ${burst_multiplier}" | bc 2>/dev/null || echo "$((target_rps / burst_multiplier))")
            # Ensure base_rps >= min_rps
            if [[ "${base_rps}" -lt "${min_rps}" ]]; then
                base_rps="${min_rps}"
            fi

            local normal_duration=$((burst_interval - burst_duration))

            echo "Burst configuration:"
            echo "  Burst: ${target_rps} RPS for ${burst_duration}s"
            echo "  Normal: ${base_rps} RPS for ${normal_duration}s"
            echo "  Cycle interval: ${burst_interval}s"

            # Boundary condition: normal period is negative
            if [[ "${normal_duration}" -le 0 ]]; then
                echo -e "${RED}ERROR: burst_interval (${burst_interval}s) must be > burst_duration (${burst_duration}s)${NC}"
                return 1
            fi

            local cycle_count=$((duration_seconds / burst_interval))

            # Boundary condition: duration < burst_interval
            # Run single cycle within duration (burst + optional normal)
            if [[ "${cycle_count}" -eq 0 ]]; then
                echo -e "${YELLOW}WARNING: duration (${duration_seconds}s) < burst_interval (${burst_interval}s). Running single cycle within duration.${NC}"
                # Fit burst and normal phases within duration
                local actual_burst_duration=$((duration_seconds < burst_duration ? duration_seconds : burst_duration))
                local actual_normal_duration=$((duration_seconds - actual_burst_duration))

                echo "  Actual burst: ${actual_burst_duration}s"
                echo "  Actual normal: ${actual_normal_duration}s"

                run_single_phase "${script_name}" "${results_base_dir}/phase_cycle_1_burst" \
                    "${target_rps}" "${actual_burst_duration}" "cycle_1_burst"

                if [[ "${actual_normal_duration}" -gt 0 ]]; then
                    run_single_phase "${script_name}" "${results_base_dir}/phase_cycle_1_normal" \
                        "${base_rps}" "${actual_normal_duration}" "cycle_1_normal"
                fi
                merge_phase_results "${results_base_dir}"
                return  # 早期リターン
            fi

            # Calculate remaining time after full cycles for final cycle adjustment
            local total_cycle_duration=$((cycle_count * burst_interval))
            local remaining_time=$((duration_seconds - total_cycle_duration))

            # Ensure remaining_time is not negative (should not happen with integer division, but guard anyway)
            if [[ "${remaining_time}" -lt 0 ]]; then
                remaining_time=0
            fi

            echo "  Total cycles: ${cycle_count}"
            if [[ "${remaining_time}" -gt 0 ]]; then
                echo "  Remaining time (added to final normal phase): ${remaining_time}s"
            fi

            for i in $(seq 1 "${cycle_count}"); do
                echo ""
                echo "--- Cycle ${i}/${cycle_count}: Burst Phase ---"
                run_single_phase "${script_name}" "${results_base_dir}/phase_cycle_${i}_burst" \
                    "${target_rps}" "${burst_duration}" "cycle_${i}_burst"

                # Final cycle uses remaining time if any
                local current_normal_duration="${normal_duration}"
                if [[ "${i}" -eq "${cycle_count}" && "${remaining_time}" -gt 0 ]]; then
                    current_normal_duration=$((normal_duration + remaining_time))
                fi

                echo ""
                echo "--- Cycle ${i}/${cycle_count}: Normal Phase (${current_normal_duration}s) ---"
                run_single_phase "${script_name}" "${results_base_dir}/phase_cycle_${i}_normal" \
                    "${base_rps}" "${current_normal_duration}" "cycle_${i}_normal"
            done

            merge_phase_results "${results_base_dir}"
            ;;

        *)
            echo -e "${YELLOW}WARNING: Unknown rps_profile '${profile}', defaulting to steady${NC}"
            run_single_phase "${script_name}" "${results_base_dir}/phase_main" \
                "${TARGET_RPS:-0}" "${duration_seconds}" "main"
            merge_phase_results "${results_base_dir}"
            ;;
    esac
}

# Run benchmarks
# Uses phased execution for all RPS profiles (steady, step_up, ramp_up_down, burst)
run_benchmark() {
    local script_name="$1"
    local script_path="${SCRIPT_DIR}/scripts/${script_name}.lua"

    if [[ ! -f "${script_path}" ]]; then
        echo -e "${YELLOW}Warning: Script not found: ${script_path}${NC}"
        return 1
    fi

    if [[ "${SCENARIO_NAME}" =~ ^tasks_update ]]; then
        if [[ "${THREADS}" != "${CONNECTIONS}" || "${WRK_THREADS:-${THREADS}}" != "${THREADS}" ]]; then
            echo -e "${RED}Error: tasks_update requires threads == connections == WRK_THREADS (current: threads=${THREADS}, connections=${CONNECTIONS}, WRK_THREADS=${WRK_THREADS:-${THREADS}})${NC}"
            echo -e "${RED}Reason: Version state management and backoff exclusion require 1:1 mapping${NC}"
            if [[ "${ALLOW_THREAD_CONNECTION_MISMATCH:-0}" != "1" ]]; then
                exit 1
            fi
            echo -e "${YELLOW}WARNING: Proceeding with degraded accuracy (ALLOW_THREAD_CONNECTION_MISMATCH=1)${NC}"
        fi
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

    # Switch RESULTS_DIR to script-specific directory for profiling, flamegraph, and meta.json
    local orig_results_dir="${RESULTS_DIR}"
    RESULTS_DIR="${script_results_dir}"

    # Build rate option for wrk2 (-R flag) for logging
    local rate_option=""
    if [[ -n "${TARGET_RPS:-}" && "${TARGET_RPS}" != "0" ]]; then
        rate_option="-R${TARGET_RPS}"
    fi

    # Log resolved parameters for debugging
    log_resolved_params "${script_name}" "${rate_option}"

    # Start profiling if enabled (now writes to script_results_dir)
    start_profiling

    # Run phased benchmark (handles all profiles: steady, step_up, ramp_up_down, burst)
    if run_phased_benchmark "${script_name}" "${script_results_dir}"; then
        # Stop profiling
        stop_profiling

        # Generate flamegraph if profiling was enabled
        generate_flamegraph

        # Extract key metrics from merged wrk.txt for summary
        local result_file="${script_results_dir}/wrk.txt"
        local reqs_sec avg_latency p50 p75 p90 p95 p99

        reqs_sec=$(grep "Requests/sec:" "${result_file}" 2>/dev/null | awk '{print $2}')
        avg_latency=$(grep "Latency" "${result_file}" 2>/dev/null | head -1 | awk '{print $2}')

        # Extract latency percentiles (P50, P75, P90, P95, P99)
        p50=$(grep -E "^[[:space:]]+50[.0-9]*%" "${result_file}" 2>/dev/null | awk '{print $2}')
        p75=$(grep -E "^[[:space:]]+75[.0-9]*%" "${result_file}" 2>/dev/null | awk '{print $2}')
        p90=$(grep -E "^[[:space:]]+90[.0-9]*%" "${result_file}" 2>/dev/null | awk '{print $2}')
        p95=$(grep -E "^[[:space:]]+95[.0-9]*%" "${result_file}" 2>/dev/null | awk '{print $2}')
        p99=$(grep -E "^[[:space:]]+99[.0-9]*%" "${result_file}" 2>/dev/null | awk '{print $2}')

        echo "" >> "${SUMMARY_FILE}"
        echo "${script_name}:" >> "${SUMMARY_FILE}"
        echo "  Requests/sec: ${reqs_sec:-N/A}" >> "${SUMMARY_FILE}"
        echo "  Avg Latency:  ${avg_latency:-N/A}" >> "${SUMMARY_FILE}"
        echo "  P50: ${p50:-N/A}" >> "${SUMMARY_FILE}"
        echo "  P75: ${p75:-N/A}" >> "${SUMMARY_FILE}"
        echo "  P90: ${p90:-N/A}" >> "${SUMMARY_FILE}"
        echo "  P95: ${p95:-N/A}" >> "${SUMMARY_FILE}"
        echo "  P99: ${p99:-N/A}" >> "${SUMMARY_FILE}"

        # Add HTTP Status Distribution (REQ-PIPELINE-005)
        local lua_metrics_summary="${script_results_dir}/lua_metrics.json"
        if [[ -f "${lua_metrics_summary}" ]] && command -v jq &> /dev/null; then
            echo "" >> "${SUMMARY_FILE}"
            echo "--- HTTP Status Distribution ---" >> "${SUMMARY_FILE}"

            # Check if http_status exists and is not empty
            local has_http_status
            has_http_status=$(jq -e '.http_status | type == "object" and length > 0' "${lua_metrics_summary}" 2>/dev/null && echo "yes" || echo "no")

            if [[ "${has_http_status}" == "yes" ]]; then
                jq -r '.http_status | to_entries[] | "  \(.key): \(.value)"' "${lua_metrics_summary}" 2>/dev/null >> "${SUMMARY_FILE}"

                echo "" >> "${SUMMARY_FILE}"
                echo "--- Error Analysis ---" >> "${SUMMARY_FILE}"

                local error_rate_summary
                error_rate_summary=$(jq -r '.error_rate // 0' "${lua_metrics_summary}" 2>/dev/null)
                if [[ -n "${error_rate_summary}" && "${error_rate_summary}" != "null" ]]; then
                    # Convert to percentage for display
                    local error_rate_percent
                    error_rate_percent=$(awk -v rate="${error_rate_summary}" 'BEGIN { printf "%.2f%%", rate * 100 }')
                    echo "  Error Rate: ${error_rate_percent}" >> "${SUMMARY_FILE}"
                fi
            else
                echo "  No HTTP status data available" >> "${SUMMARY_FILE}"
            fi
        fi

        # Add phase count if multi-phase execution
        if [[ -n "${MERGED_PHASE_COUNT:-}" && "${MERGED_PHASE_COUNT}" -gt 1 ]]; then
            echo "  Phases: ${MERGED_PHASE_COUNT}" >> "${SUMMARY_FILE}"
        fi

        # Generate meta.json in script-specific directory
        generate_meta_json "${result_file}" "${script_name}"

        # Generate meta_extended.json for phased execution details
        generate_meta_extended "${RESULTS_DIR}"

        # Restore RESULTS_DIR
        RESULTS_DIR="${orig_results_dir}"

        echo -e "${GREEN}Completed${NC}"
    else
        # Stop profiling even on failure
        stop_profiling

        # Restore RESULTS_DIR
        RESULTS_DIR="${orig_results_dir}"

        echo -e "${RED}Failed${NC}"
        echo "${script_name}: FAILED" >> "${SUMMARY_FILE}"
        return 1
    fi
}

# Get list of scripts to run
# Priority: SPECIFIC_SCRIPT (CLI arg) > scenario endpoint mapping > legacy fallback
if [[ -n "${SPECIFIC_SCRIPT}" ]]; then
    # Explicit script specified via CLI
    SCRIPTS=("${SPECIFIC_SCRIPT}")
    echo "Running specific script: ${SPECIFIC_SCRIPT}"
else
    # Resolve scripts from scenario configuration
    echo "Resolving scripts from scenario..."
    resolved_scripts=$(resolve_scripts_from_scenario "${SCENARIO_FILE}")

    # Convert space-separated string to array
    read -ra SCRIPTS <<< "${resolved_scripts}"

    if [[ ${#SCRIPTS[@]} -eq 0 ]]; then
        echo -e "${RED}Error: No scripts resolved from scenario${NC}"
        exit 1
    fi

    echo "Scripts to run: ${SCRIPTS[*]}"
fi

# Run all benchmarks and track failures
FAILED_SCRIPTS=()
for script in "${SCRIPTS[@]}"; do
    if ! run_benchmark "${script}"; then
        FAILED_SCRIPTS+=("${script}")
    fi
done

# Generate bottleneck analysis (executed before summary display)
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
    echo "Slowest endpoint: ${slowest_endpoint} (${slowest_rps} req/s)" >> "${SUMMARY_FILE}"
fi

if [ -n "$highest_p99_endpoint" ]; then
    echo "Highest P99 latency: ${highest_p99_endpoint}" >> "${SUMMARY_FILE}"
fi

# =============================================================================
# Schema Validation (REQ-PROFILE-JSON-001)
# =============================================================================
# Validate all generated meta.json files against the schema.
# This ensures that JSON output is valid and conforms to the expected format.
# Validation failure is treated as a benchmark failure.
# Performance requirement: validation overhead must not exceed 5% of total execution time
# =============================================================================

VALIDATE_SCRIPT="${SCRIPT_DIR}/validate_meta_schema.sh"

# Check if validation script exists and is executable (mandatory requirement)
if [[ ! -x "${VALIDATE_SCRIPT}" ]]; then
    echo "" >> "${SUMMARY_FILE}"
    echo "--- Schema Validation ---" >> "${SUMMARY_FILE}"
    echo "Error: Schema validation script not found or not executable" >> "${SUMMARY_FILE}"
    FAILED_SCRIPTS+=("(validation_script_missing)")
else
    # Measure validation overhead (must not exceed 5% of total execution time)
    # Use millisecond precision to avoid TOTAL_TIME=0 for short executions
    VALIDATION_START=$(get_timestamp_ms)

    # Run schema validation with --all flag for recursive search
    # This validates all meta.json files in ${RESULTS_DIR} and subdirectories
    # Capture validation output to include details in summary on failure
    # Use portable mktemp syntax (works on both GNU/Linux and BSD/macOS)
    MKTEMP_ERROR=$(mktemp -t validation.XXXXXX 2>&1) && VALIDATION_OUTPUT="${MKTEMP_ERROR}"
    if [[ ! -f "${VALIDATION_OUTPUT}" ]]; then
        echo "" >> "${SUMMARY_FILE}"
        echo "--- Schema Validation ---" >> "${SUMMARY_FILE}"
        echo "Error: Failed to create temporary file for validation output" >> "${SUMMARY_FILE}"
        echo "mktemp error: ${MKTEMP_ERROR}" >> "${SUMMARY_FILE}"
        FAILED_SCRIPTS+=("(validation_mktemp_failed)")
    elif "${VALIDATE_SCRIPT}" --all "${RESULTS_DIR}" > "${VALIDATION_OUTPUT}" 2>&1; then
        VALIDATION_END=$(get_timestamp_ms)
        VALIDATION_TIME=$((VALIDATION_END - VALIDATION_START))

        echo "" >> "${SUMMARY_FILE}"
        echo "--- Schema Validation ---" >> "${SUMMARY_FILE}"
        echo "Schema validation passed (${VALIDATION_TIME}ms)" >> "${SUMMARY_FILE}"

        # Calculate validation overhead percentage
        # Use BENCHMARK_START_TIME if available (set at script start)
        if [[ -n "${BENCHMARK_START_TIME:-}" ]]; then
            BENCHMARK_END=$(get_timestamp_ms)
            TOTAL_TIME=$((BENCHMARK_END - BENCHMARK_START_TIME))
            if [[ ${TOTAL_TIME} -gt 0 ]]; then
                # Calculate overhead percentage (for display)
                VALIDATION_OVERHEAD_PCT=$(awk -v vt="${VALIDATION_TIME}" -v tt="${TOTAL_TIME}" \
                    'BEGIN { printf "%.2f", (vt / tt) * 100 }')
                echo "Validation overhead: ${VALIDATION_OVERHEAD_PCT}%" >> "${SUMMARY_FILE}"

                # Check if overhead exceeds 5% threshold using awk to avoid integer overflow
                # Compare: validation_time / total_time >= 0.05 (5%)
                # Using awk for floating-point comparison (handles large values without overflow)
                if awk -v vt="${VALIDATION_TIME}" -v tt="${TOTAL_TIME}" \
                    'BEGIN { exit (vt / tt >= 0.05) ? 0 : 1 }'; then
                    echo "Error: Validation overhead (${VALIDATION_OVERHEAD_PCT}%) meets or exceeds 5% threshold" >> "${SUMMARY_FILE}"
                    FAILED_SCRIPTS+=("(validation_overhead_exceeded)")
                fi
            fi
        fi
    else
        VALIDATION_END=$(get_timestamp_ms)
        VALIDATION_TIME=$((VALIDATION_END - VALIDATION_START))

        # Include validation error details in summary for debugging
        # Strip ANSI color codes to keep summary.txt clean
        echo "" >> "${SUMMARY_FILE}"
        echo "--- Schema Validation ---" >> "${SUMMARY_FILE}"
        echo "Schema validation failed (${VALIDATION_TIME}ms)" >> "${SUMMARY_FILE}"
        echo "" >> "${SUMMARY_FILE}"
        echo "Validation errors (first 10 lines):" >> "${SUMMARY_FILE}"
        # Strip ANSI color codes using Perl (more portable than sed for this case)
        if command -v perl >/dev/null 2>&1; then
            perl -pe 's/\e\[[0-9;]*m//g' "${VALIDATION_OUTPUT}" | head -10 >> "${SUMMARY_FILE}" 2>/dev/null || true
        else
            # Fallback: include with color codes if perl is not available
            head -10 "${VALIDATION_OUTPUT}" >> "${SUMMARY_FILE}" 2>/dev/null || true
        fi

        FAILED_SCRIPTS+=("(schema_validation)")
    fi

    # Cleanup temporary validation output file (only if it exists)
    [[ -n "${VALIDATION_OUTPUT:-}" && -f "${VALIDATION_OUTPUT}" ]] && rm -f "${VALIDATION_OUTPUT}"
fi

# Display summary after all processing (including validation) is complete
echo ""
echo "=============================================="
echo "  Benchmark Complete"
echo "=============================================="
echo ""
echo "Results saved to: ${RESULTS_DIR}"
echo ""
echo "Summary:"
cat "${SUMMARY_FILE}"

# Bottleneck analysis output (already written to summary.txt above)
if [ -n "$slowest_endpoint" ]; then
    echo ""
    echo -e "${YELLOW}Slowest endpoint: ${slowest_endpoint} (${slowest_rps} req/s)${NC}"
fi

if [ -n "$highest_p99_endpoint" ]; then
    echo -e "${YELLOW}Highest P99 latency: ${highest_p99_endpoint}${NC}"
fi
echo ""

# Report failed scripts and exit with error if any failures
if [[ ${#FAILED_SCRIPTS[@]} -gt 0 ]]; then
    echo -e "${RED}=============================================="
    echo "  Benchmark Failures Detected"
    echo "==============================================${NC}"
    echo ""
    echo -e "${RED}Failed scripts:${NC}"
    for script in "${FAILED_SCRIPTS[@]}"; do
        echo "  - ${script}"
    done
    echo ""
    exit 1
fi
