#!/bin/bash
# Seed benchmark data
# benches/api/benchmarks/scripts/seed.sh
#
# Seeds benchmark data using the seed_data.lua script.
#
# Usage:
#   ./seed.sh [options]
#
# Options:
#   --scenario <yaml>      Use scenario file for configuration
#   --scale <scale>        Data scale (small, medium, large)
#   --count <number>       Explicit record count
#   --seed <number>        Random seed for reproducibility
#   --incremental          Add to existing data (do not clear)
#   --endpoint <url>       API endpoint (default: http://localhost:8080)
#   --variant <variant>    Payload variant (minimal, standard, complex, heavy)
#   --batch-size <number>  Records per batch (default: 100)
#   --help                 Show this help message
#
# Priority: CLI options > Scenario file > Defaults
#
# Examples:
#   # Basic seeding with medium scale
#   ./seed.sh --scale medium
#
#   # Large scale with specific seed for reproducibility
#   ./seed.sh --scale large --seed 42
#
#   # Use scenario file for configuration
#   ./seed.sh --scenario ../scenarios/large_scale_seeded.yaml
#
#   # Scenario file but override scale from CLI
#   ./seed.sh --scenario ../scenarios/large_scale_seeded.yaml --scale small
#
#   # Custom record count
#   ./seed.sh --count 50000 --seed 12345
#
#   # Incremental seeding (preserve existing data)
#   ./seed.sh --scale medium --incremental

set -euo pipefail

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default values
DEFAULT_SCALE="medium"
DEFAULT_ENDPOINT="http://localhost:8080"
DEFAULT_VARIANT="standard"
DEFAULT_BATCH_SIZE="100"

# Current values (start with defaults)
SCALE="${DEFAULT_SCALE}"
COUNT=""
SEED=""
INCREMENTAL=""
ENDPOINT="${DEFAULT_ENDPOINT}"
SCENARIO_FILE=""
VARIANT="${DEFAULT_VARIANT}"
BATCH_SIZE="${DEFAULT_BATCH_SIZE}"

# Track which options were explicitly set via CLI
CLI_SCALE=""
CLI_COUNT=""
CLI_SEED=""
CLI_VARIANT=""
CLI_INCREMENTAL=""
CLI_ENDPOINT=""
CLI_BATCH_SIZE=""

# Show help message
show_help() {
    sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# //' | sed 's/^#$//'
}

# Parse arguments and track CLI-specified values
while [[ $# -gt 0 ]]; do
    case $1 in
        --scenario)
            SCENARIO_FILE="$2"
            shift 2
            ;;
        --scale)
            CLI_SCALE="$2"
            SCALE="$2"
            shift 2
            ;;
        --count)
            CLI_COUNT="$2"
            COUNT="$2"
            shift 2
            ;;
        --seed)
            CLI_SEED="$2"
            SEED="$2"
            shift 2
            ;;
        --incremental)
            CLI_INCREMENTAL="1"
            INCREMENTAL="1"
            shift
            ;;
        --endpoint)
            CLI_ENDPOINT="$2"
            ENDPOINT="$2"
            shift 2
            ;;
        --variant)
            CLI_VARIANT="$2"
            VARIANT="$2"
            shift 2
            ;;
        --batch-size)
            CLI_BATCH_SIZE="$2"
            BATCH_SIZE="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            exit 0
            ;;
        *)
            echo "Error: Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Load from scenario file if provided
# Only apply scenario values for options NOT explicitly set via CLI
if [[ -n "${SCENARIO_FILE}" ]]; then
    if [[ ! -f "${SCENARIO_FILE}" ]]; then
        echo "Error: Scenario file not found: ${SCENARIO_FILE}"
        exit 1
    fi

    echo "Loading configuration from scenario file: ${SCENARIO_FILE}"

    # Check if yq is available
    if ! command -v yq &> /dev/null; then
        echo "Error: yq is required to parse YAML files"
        echo "Install with: brew install yq (macOS) or apt-get install yq (Ubuntu)"
        exit 1
    fi

    # Extract data_scale_config if present, otherwise use data_scale
    CONFIG_SCALE=$(yq '.data_scale_config.scale // .data_scale // "medium"' "${SCENARIO_FILE}" 2>/dev/null || echo "medium")
    CONFIG_COUNT=$(yq '.data_scale_config.record_count // null' "${SCENARIO_FILE}" 2>/dev/null || echo "null")
    CONFIG_SEED=$(yq '.data_scale_config.seed // null' "${SCENARIO_FILE}" 2>/dev/null || echo "null")
    CONFIG_INCREMENTAL=$(yq '.data_scale_config.incremental // false' "${SCENARIO_FILE}" 2>/dev/null || echo "false")
    CONFIG_VARIANT=$(yq '.payload_variant // "standard"' "${SCENARIO_FILE}" 2>/dev/null || echo "standard")
    CONFIG_ENDPOINT=$(yq '.endpoint // null' "${SCENARIO_FILE}" 2>/dev/null || echo "null")
    CONFIG_BATCH_SIZE=$(yq '.data_scale_config.batch_size // null' "${SCENARIO_FILE}" 2>/dev/null || echo "null")

    # Apply scenario values ONLY if NOT overridden by CLI
    # This ensures CLI > Scenario > Default priority
    if [[ -z "${CLI_SCALE}" ]]; then
        SCALE="${CONFIG_SCALE}"
    fi
    if [[ -z "${CLI_COUNT}" && "${CONFIG_COUNT}" != "null" ]]; then
        COUNT="${CONFIG_COUNT}"
    fi
    if [[ -z "${CLI_SEED}" && "${CONFIG_SEED}" != "null" ]]; then
        SEED="${CONFIG_SEED}"
    fi
    if [[ -z "${CLI_INCREMENTAL}" && "${CONFIG_INCREMENTAL}" == "true" ]]; then
        INCREMENTAL="1"
    fi
    if [[ -z "${CLI_VARIANT}" ]]; then
        VARIANT="${CONFIG_VARIANT}"
    fi
    if [[ -z "${CLI_ENDPOINT}" && "${CONFIG_ENDPOINT}" != "null" ]]; then
        ENDPOINT="${CONFIG_ENDPOINT}"
    fi
    if [[ -z "${CLI_BATCH_SIZE}" && "${CONFIG_BATCH_SIZE}" != "null" ]]; then
        BATCH_SIZE="${CONFIG_BATCH_SIZE}"
    fi
fi

# Calculate record count from scale if not specified
if [[ -z "${COUNT}" ]]; then
    case "${SCALE}" in
        small)  COUNT=1000 ;;
        medium) COUNT=10000 ;;
        large)  COUNT=1000000 ;;
        *)
            echo "Error: Invalid scale: ${SCALE}"
            echo "Valid values: small, medium, large"
            exit 1
            ;;
    esac
fi

# Print configuration
echo ""
echo "=== Seeding Configuration ==="
echo "Scale:          ${SCALE}"
echo "Record count:   ${COUNT}"
echo "Seed:           ${SEED:-random}"
echo "Incremental:    ${INCREMENTAL:-no}"
echo "Endpoint:       ${ENDPOINT}"
echo "Payload variant: ${VARIANT}"
echo "Batch size:     ${BATCH_SIZE}"
if [[ -n "${SCENARIO_FILE}" ]]; then
    echo "Scenario file:  ${SCENARIO_FILE}"
fi
echo "============================="
echo ""

# Set environment variables for the Lua script
export DATA_SCALE="${SCALE}"
export RECORD_COUNT="${COUNT}"
export API_ENDPOINT="${ENDPOINT}"
export PAYLOAD_VARIANT="${VARIANT}"
export BATCH_SIZE="${BATCH_SIZE}"

if [[ -n "${SEED}" ]]; then
    export RANDOM_SEED="${SEED}"
fi

if [[ -n "${INCREMENTAL}" ]]; then
    export INCREMENTAL="1"
fi

# Run the seeding script
echo "Starting data seeding..."
cd "${SCRIPT_DIR}"

# Check if lua is available
if command -v lua &> /dev/null; then
    lua -e "require('seed_data').run()"
elif command -v lua5.3 &> /dev/null; then
    lua5.3 -e "require('seed_data').run()"
elif command -v lua5.4 &> /dev/null; then
    lua5.4 -e "require('seed_data').run()"
else
    echo "Error: Lua interpreter not found"
    echo "Install with: brew install lua (macOS) or apt-get install lua5.3 (Ubuntu)"
    exit 1
fi

echo ""
echo "Seeding complete!"

# =============================================================================
# Generate seed_meta.json
# =============================================================================

generate_seed_meta() {
    local meta_file="${SCRIPT_DIR}/seed_meta.json"
    local timestamp
    timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    # Convert scale to scientific notation for consistency
    local data_scale_notation
    case "${SCALE}" in
        small)  data_scale_notation="1e2" ;;
        medium) data_scale_notation="1e4" ;;
        large)  data_scale_notation="1e6" ;;
        *)      data_scale_notation="${SCALE}" ;;
    esac

    # Estimate project count (typically 1% of task count)
    local project_count
    project_count=$((COUNT / 100))
    if [[ ${project_count} -lt 1 ]]; then
        project_count=1
    fi

    # Calculate expected hit rate based on data scale
    # Higher scale = higher hit rate due to cache efficiency
    local expected_hit_rate
    case "${SCALE}" in
        small)  expected_hit_rate="0.3" ;;
        medium) expected_hit_rate="0.5" ;;
        large)  expected_hit_rate="0.8" ;;
        *)      expected_hit_rate="0.5" ;;
    esac

    # Convert INCREMENTAL to boolean
    local incremental_bool="false"
    if [[ -n "${INCREMENTAL}" ]]; then
        incremental_bool="true"
    fi

    # Validate and prepare seed_value (must be numeric or null)
    local seed_value="null"
    if [[ -n "${SEED}" ]]; then
        if [[ "${SEED}" =~ ^[0-9]+$ ]]; then
            seed_value="${SEED}"
        else
            echo "Warning: SEED '${SEED}' is not a valid number, using null"
        fi
    fi

    # Check if jq is available for safe JSON generation
    if command -v jq &> /dev/null; then
        # Use jq for proper JSON escaping
        jq -n \
            --arg seeded_at "${timestamp}" \
            --arg data_scale "${data_scale_notation}" \
            --arg payload_variant "${VARIANT}" \
            --argjson tasks "${COUNT}" \
            --argjson projects "${project_count}" \
            --argjson expected_hit_rate "${expected_hit_rate}" \
            --argjson seed_value "${seed_value}" \
            --argjson incremental "${incremental_bool}" \
            --argjson batch_size "${BATCH_SIZE}" \
            --arg endpoint "${ENDPOINT}" \
            '{
                seeded_at: $seeded_at,
                data_scale: $data_scale,
                payload_variant: $payload_variant,
                records: {
                    tasks: $tasks,
                    projects: $projects
                },
                expected_hit_rate: $expected_hit_rate,
                seed_value: $seed_value,
                incremental: $incremental,
                batch_size: $batch_size,
                endpoint: $endpoint
            }' > "${meta_file}" || {
                echo "Error: Failed to write ${meta_file}"
                exit 1
            }
    else
        # Fallback: manual JSON generation with escaping
        # Use perl if available for comprehensive control character escaping
        # Otherwise use basic sed escaping with warning
        local escaped_variant
        local escaped_endpoint

        if command -v perl &> /dev/null; then
            # Perl: escape all JSON-unsafe characters (backslash, quote, control chars 0x00-0x1F)
            escaped_variant=$(printf '%s' "${VARIANT}" | perl -pe 's/\\/\\\\/g; s/"/\\"/g; s/[\x00-\x1f]/sprintf("\\u%04x", ord($&))/ge')
            escaped_endpoint=$(printf '%s' "${ENDPOINT}" | perl -pe 's/\\/\\\\/g; s/"/\\"/g; s/[\x00-\x1f]/sprintf("\\u%04x", ord($&))/ge')
        else
            # Basic escaping: backslash, double quote, replace control chars with space
            echo "Warning: Neither jq nor perl available. Using basic escaping for seed_meta.json"
            escaped_variant=$(printf '%s' "${VARIANT}" | sed 's/\\/\\\\/g; s/"/\\"/g' | tr '\000-\037' ' ')
            escaped_endpoint=$(printf '%s' "${ENDPOINT}" | sed 's/\\/\\\\/g; s/"/\\"/g' | tr '\000-\037' ' ')
        fi

        cat > "${meta_file}" << EOF || {
            echo "Error: Failed to write ${meta_file}"
            exit 1
        }
{
  "seeded_at": "${timestamp}",
  "data_scale": "${data_scale_notation}",
  "payload_variant": "${escaped_variant}",
  "records": {
    "tasks": ${COUNT},
    "projects": ${project_count}
  },
  "expected_hit_rate": ${expected_hit_rate},
  "seed_value": ${seed_value},
  "incremental": ${incremental_bool},
  "batch_size": ${BATCH_SIZE},
  "endpoint": "${escaped_endpoint}"
}
EOF
    fi

    echo ""
    echo "Generated: ${meta_file}"
    echo ""
    cat "${meta_file}"
}

generate_seed_meta
