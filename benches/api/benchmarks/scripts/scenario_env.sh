#!/usr/bin/env bash
# Scenario environment variable loading utilities
#
# This script is designed to be sourced, not executed directly.
# It provides functions to load scenario configuration from YAML files.
#
# Note: This script intentionally does not set `set -euo pipefail` because
# it is meant to be sourced into other scripts. Error handling is done
# through return values and explicit checks. The sourcing script should
# set its own error handling options.
#
# Usage:
#   source scripts/scenario_env.sh
#   load_scenario_env "path/to/scenario.yaml"
#   export_scenario_env
#   display_scenario_env

# Prevent direct execution
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    echo "Error: This script should be sourced, not executed directly."
    echo "Usage: source scripts/scenario_env.sh"
    exit 1
fi

# Scenario variable storage
_SCENARIO_NAME=""
_STORAGE_MODE=""
_CACHE_MODE=""
_DATA_SCALE=""
_HIT_RATE=""
_CACHE_STRATEGY=""
_RPS_PROFILE=""
_THREADS=""
_CONNECTIONS=""
_DURATION=""
_ENDPOINT=""
_PAYLOAD=""
_DATABASE_POOL_SIZE=""
_REDIS_POOL_SIZE=""
_WORKER_THREADS=""
_RETRY=""
_FAIL_RATE=""

# Parse YAML value with yq (preferred) or fallback to grep/sed
parse_yaml_value() {
    local file="$1"
    local key="$2"
    local default="${3:-}"
    local required="${4:-false}"

    [[ ! -f "${file}" ]] && { echo "${default}"; return 1; }

    local value
    if command -v yq &> /dev/null; then
        if [[ "${required}" == "true" ]]; then
            value=$(yq -e ".${key}" "${file}" 2>/dev/null | tr -d '"') || { echo "Error: Required key '${key}' not found in ${file}" >&2; return 1; }
        else
            value=$(yq ".${key} // \"${default}\"" "${file}" 2>/dev/null | tr -d '"')
        fi
    else
        value=$(grep "^${key}:" "${file}" 2>/dev/null | head -1 | sed 's/^[^:]*: *//' | tr -d '"' || echo "${default}")
        if [[ "${required}" == "true" && -z "${value}" ]]; then
            echo "Error: Required key '${key}' not found in ${file} (yq not available for strict validation)" >&2
            return 1
        fi
    fi

    [[ -z "${value}" || "${value}" == "null" ]] && echo "${default}" || echo "${value}"
}

# Map data_size to DATA_SCALE
map_data_scale() {
    case "$1" in
        small) echo "1e2" ;;
        medium) echo "1e4" ;;
        large) echo "1e6" ;;
        *) echo "$1" ;;
    esac
}

# Map payload complexity to payload size
map_payload() {
    case "$1" in
        minimal|small) echo "small" ;;
        standard|medium) echo "medium" ;;
        complex|large|heavy) echo "large" ;;
        *) echo "$1" ;;
    esac
}

# Map load_pattern to RPS_PROFILE
map_rps_profile() {
    case "$1" in
        constant|steady) echo "steady" ;;
        ramp_up_down) echo "ramp_up_down" ;;
        burst) echo "burst" ;;
        step_up) echo "step_up" ;;
        *) echo "$1" ;;
    esac
}

# Load environment variables from scenario YAML
load_scenario_env() {
    local scenario_file="$1"

    [[ ! -f "${scenario_file}" ]] && { echo "Error: Scenario file not found: ${scenario_file}" >&2; return 1; }

    # REQ-BENCH-002: Support new scenario keys with backward compatibility
    _SCENARIO_NAME=$(parse_yaml_value "${scenario_file}" "name" "benchmark" "true") || return 1

    # Storage: try 'storage' (new) then 'storage_mode' (legacy)
    _STORAGE_MODE=$(parse_yaml_value "${scenario_file}" "storage" "")
    [[ -z "${_STORAGE_MODE}" ]] && _STORAGE_MODE=$(parse_yaml_value "${scenario_file}" "storage_mode" "in_memory")

    # Cache: try 'cache' (new) then 'cache_mode' (legacy)
    _CACHE_MODE=$(parse_yaml_value "${scenario_file}" "cache" "")
    [[ -z "${_CACHE_MODE}" ]] && _CACHE_MODE=$(parse_yaml_value "${scenario_file}" "cache_mode" "none")

    # Data scale: try 'data_scale' (new) then 'data_size' (legacy with mapping)
    _DATA_SCALE=$(parse_yaml_value "${scenario_file}" "data_scale" "")
    if [[ -z "${_DATA_SCALE}" ]]; then
        local data_size
        data_size=$(parse_yaml_value "${scenario_file}" "data_size" "small")
        _DATA_SCALE=$(map_data_scale "${data_size}")
    fi

    _HIT_RATE=$(parse_yaml_value "${scenario_file}" "hit_rate" "50")
    _CACHE_STRATEGY=$(parse_yaml_value "${scenario_file}" "cache_strategy" "read-through")

    # RPS profile: try 'rps_profile' (new) then 'load_pattern' (legacy with mapping)
    _RPS_PROFILE=$(parse_yaml_value "${scenario_file}" "rps_profile" "")
    if [[ -z "${_RPS_PROFILE}" ]]; then
        local load_pattern
        load_pattern=$(parse_yaml_value "${scenario_file}" "load_pattern" "steady")
        _RPS_PROFILE=$(map_rps_profile "${load_pattern}")
    fi

    _THREADS=$(parse_yaml_value "${scenario_file}" "threads" "2")
    _CONNECTIONS=$(parse_yaml_value "${scenario_file}" "connections" "10")
    _DURATION=$(parse_yaml_value "${scenario_file}" "duration_seconds" "30")
    _ENDPOINT=$(parse_yaml_value "${scenario_file}" "endpoint" "/tasks")
    _PAYLOAD=$(map_payload "$(parse_yaml_value "${scenario_file}" "payload" "medium")")
    _DATABASE_POOL_SIZE=$(parse_yaml_value "${scenario_file}" "database_pool_size" "16")
    _REDIS_POOL_SIZE=$(parse_yaml_value "${scenario_file}" "redis_pool_size" "8")
    _WORKER_THREADS=$(parse_yaml_value "${scenario_file}" "worker_threads" "4")
    _RETRY=$(parse_yaml_value "${scenario_file}" "retry" "false")
    _FAIL_RATE=$(parse_yaml_value "${scenario_file}" "fail_rate" "0")
}

# Export loaded scenario variables to environment
export_scenario_env() {
    export SCENARIO_NAME="${_SCENARIO_NAME}"
    export STORAGE_MODE="${_STORAGE_MODE}"
    export CACHE_MODE="${_CACHE_MODE}"
    export DATA_SCALE="${_DATA_SCALE}"
    export HIT_RATE="${_HIT_RATE}"
    export CACHE_STRATEGY="${_CACHE_STRATEGY}"
    export RPS_PROFILE="${_RPS_PROFILE}"
    export THREADS="${_THREADS}"
    export CONNECTIONS="${_CONNECTIONS}"
    export DURATION="${_DURATION}"
    export ENDPOINT="${_ENDPOINT}"
    export PAYLOAD="${_PAYLOAD}"
    export DATABASE_POOL_SIZE="${_DATABASE_POOL_SIZE}"
    export REDIS_POOL_SIZE="${_REDIS_POOL_SIZE}"
    export WORKER_THREADS="${_WORKER_THREADS}"
    export RETRY="${_RETRY}"
    export FAIL_RATE="${_FAIL_RATE}"
}

# Display loaded scenario variables
display_scenario_env() {
    cat <<EOF
Loaded scenario environment variables:
  SCENARIO_NAME=${_SCENARIO_NAME}
  STORAGE_MODE=${_STORAGE_MODE}
  CACHE_MODE=${_CACHE_MODE}
  DATA_SCALE=${_DATA_SCALE}
  HIT_RATE=${_HIT_RATE}
  CACHE_STRATEGY=${_CACHE_STRATEGY}
  RPS_PROFILE=${_RPS_PROFILE}
  THREADS=${_THREADS}
  CONNECTIONS=${_CONNECTIONS}
  DURATION=${_DURATION}
  ENDPOINT=${_ENDPOINT}
  PAYLOAD=${_PAYLOAD}
  DATABASE_POOL_SIZE=${_DATABASE_POOL_SIZE}
  REDIS_POOL_SIZE=${_REDIS_POOL_SIZE}
  WORKER_THREADS=${_WORKER_THREADS}
  RETRY=${_RETRY}
  FAIL_RATE=${_FAIL_RATE}
EOF
}
