#!/bin/bash
# benches/api/benchmarks/setup_test_data.sh
#
# Create test data for benchmark and save IDs to a file
#
# Usage:
#   ./setup_test_data.sh                                   # Setup with defaults
#   ./setup_test_data.sh --scale medium                    # 10,000 tasks
#   ./setup_test_data.sh --scale large                     # 1,000,000 tasks
#   ./setup_test_data.sh --payload complex                 # Complex payloads
#   ./setup_test_data.sh --seed 42                         # Reproducible data
#   ./setup_test_data.sh --meta-output setup_meta.json     # Output metadata
#   API_URL=http://localhost:3001 ./setup_test_data.sh     # Custom URL
#
# Options:
#   --scale <small|medium|large>    Data scale (default: small)
#                                   small: 100 tasks, 10 projects
#                                   medium: 10,000 tasks, 100 projects
#                                   large: 1,000,000 tasks, 1,000 projects
#   --payload <minimal|standard|complex|heavy>  Payload variant (default: standard)
#   --seed <number>                 Random seed for reproducible data
#   --meta-output <file>            Output setup metadata to JSON file
#   --batch-size <number>           Batch size for bulk creation (default: 100)
#   --help                          Show this help message
#
# Environment Variables:
#   API_URL                         API server URL (default: http://localhost:3002)

set -euo pipefail

# =============================================================================
# Configuration
# =============================================================================

API_URL="${API_URL:-http://localhost:3002}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IDS_FILE="${SCRIPT_DIR}/scripts/test_ids.lua"

# Default values
DATA_SCALE="small"
PAYLOAD_VARIANT="standard"
SEED=""
META_OUTPUT=""
BATCH_SIZE=100

# Scale mappings
declare -A TASK_COUNTS=(
    ["small"]=100
    ["medium"]=10000
    ["large"]=1000000
)
declare -A PROJECT_COUNTS=(
    ["small"]=10
    ["medium"]=100
    ["large"]=1000
)

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# =============================================================================
# Argument Parsing
# =============================================================================

show_help() {
    sed -n '3,28p' "$0" | sed 's/^# //' | sed 's/^#//'
    exit 0
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --scale)
            DATA_SCALE="$2"
            shift 2
            ;;
        --payload)
            PAYLOAD_VARIANT="$2"
            shift 2
            ;;
        --seed)
            SEED="$2"
            shift 2
            ;;
        --meta-output)
            META_OUTPUT="$2"
            shift 2
            ;;
        --batch-size)
            BATCH_SIZE="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            show_help
            ;;
    esac
done

# =============================================================================
# Validation
# =============================================================================

# Validate data scale
if [[ ! "${TASK_COUNTS[$DATA_SCALE]+_}" ]]; then
    echo -e "${RED}Error: Invalid scale '${DATA_SCALE}'. Must be: small | medium | large${NC}"
    exit 1
fi

# Validate payload variant
case "${PAYLOAD_VARIANT}" in
    minimal|standard|complex|heavy) ;;
    *)
        echo -e "${RED}Error: Invalid payload '${PAYLOAD_VARIANT}'. Must be: minimal | standard | complex | heavy${NC}"
        exit 1
        ;;
esac

# Validate batch size
if ! [[ "${BATCH_SIZE}" =~ ^[0-9]+$ ]] || [[ "${BATCH_SIZE}" -lt 1 ]]; then
    echo -e "${RED}Error: Invalid batch size '${BATCH_SIZE}'. Must be a positive integer${NC}"
    exit 1
fi

# Validate seed (must be numeric if provided)
if [[ -n "${SEED}" ]] && ! [[ "${SEED}" =~ ^[0-9]+$ ]]; then
    echo -e "${RED}Error: Invalid seed '${SEED}'. Must be a positive integer${NC}"
    exit 1
fi

# =============================================================================
# Payload Generation
# =============================================================================

# Generate a deterministic random string based on seed and index
generate_seeded_string() {
    local length="$1"
    local index="$2"
    if [[ -n "${SEED}" ]]; then
        # Use seed + index to generate deterministic string
        # Repeat hash generation to ensure we meet the required length
        local result=""
        local counter=0
        while [[ ${#result} -lt ${length} ]]; do
            local hash
            hash=$(echo "${SEED}${index}${counter}" | md5sum | cut -c1-32)
            result="${result}${hash}"
            counter=$((counter + 1))
        done
        echo "${result:0:${length}}"
    else
        # Random string (use head -c to avoid fold buffering issues in CI)
        head -c 256 /dev/urandom | LC_ALL=C tr -dc 'a-zA-Z0-9' | head -c "${length}"
        echo ""  # Ensure newline
    fi
}

# Generate task payload based on variant
generate_task_payload() {
    local index="$1"
    local title_suffix
    title_suffix=$(generate_seeded_string 8 "$index")

    case "${PAYLOAD_VARIANT}" in
        minimal)
            echo "{\"title\":\"Task ${index}\"}"
            ;;
        standard)
            echo "{\"title\":\"Benchmark Task ${index}-${title_suffix}\",\"description\":\"Test task for benchmarking\",\"priority\":\"medium\",\"tags\":[\"benchmark\",\"test\"]}"
            ;;
        complex)
            local desc
            desc=$(generate_seeded_string 200 "$index")
            echo "{\"title\":\"Complex Task ${index}-${title_suffix}\",\"description\":\"${desc}\",\"priority\":\"high\",\"tags\":[\"benchmark\",\"test\",\"complex\",\"performance\"],\"metadata\":{\"index\":${index},\"variant\":\"complex\"}}"
            ;;
        heavy)
            local desc
            desc=$(generate_seeded_string 1000 "$index")
            local notes
            notes=$(generate_seeded_string 2000 "$index")
            echo "{\"title\":\"Heavy Task ${index}-${title_suffix}\",\"description\":\"${desc}\",\"priority\":\"critical\",\"tags\":[\"benchmark\",\"test\",\"heavy\",\"performance\",\"load\"],\"metadata\":{\"index\":${index},\"variant\":\"heavy\",\"notes\":\"${notes}\"}}"
            ;;
    esac
}

# Generate project payload based on variant
generate_project_payload() {
    local index="$1"
    local name_suffix
    name_suffix=$(generate_seeded_string 8 "$index")

    case "${PAYLOAD_VARIANT}" in
        minimal)
            echo "{\"name\":\"Project ${index}\"}"
            ;;
        standard)
            echo "{\"name\":\"Benchmark Project ${index}-${name_suffix}\",\"description\":\"Test project for benchmarking\"}"
            ;;
        complex|heavy)
            local desc
            desc=$(generate_seeded_string 500 "$index")
            echo "{\"name\":\"Project ${index}-${name_suffix}\",\"description\":\"${desc}\",\"metadata\":{\"index\":${index},\"variant\":\"${PAYLOAD_VARIANT}\"}}"
            ;;
    esac
}

# =============================================================================
# Data Creation Functions
# =============================================================================

# Create tasks using bulk API
create_tasks_bulk() {
    local total="$1"
    local created=0
    local batch_num=0

    echo "Creating ${total} tasks (batch size: ${BATCH_SIZE})..."

    while [[ ${created} -lt ${total} ]]; do
        local batch_count=$((total - created))
        if [[ ${batch_count} -gt ${BATCH_SIZE} ]]; then
            batch_count=${BATCH_SIZE}
        fi

        batch_num=$((batch_num + 1))
        echo ""  # Force newline for CI log visibility
        echo "[DEBUG] $(date '+%H:%M:%S') Batch ${batch_num}: preparing ${batch_count} tasks..."

        # Generate batch payload
        local tasks_json="["
        for ((i = 0; i < batch_count; i++)); do
            local task_index=$((created + i + 1))
            if [[ $i -gt 0 ]]; then
                tasks_json+=","
            fi
            tasks_json+=$(generate_task_payload "${task_index}")
        done
        tasks_json+="]"

        echo "[DEBUG] $(date '+%H:%M:%S') Payload generated (${#tasks_json} bytes), calling curl..."

        # Send bulk request with HTTP status code capture
        local response
        local http_code
        echo "[DEBUG] $(date '+%H:%M:%S') curl -s --connect-timeout 5 --max-time 10 -X POST ${API_URL}/tasks/bulk"
        response=$(curl -s --connect-timeout 5 --max-time 10 -w "\n%{http_code}" \
            -X POST "${API_URL}/tasks/bulk" \
            -H "Content-Type: application/json" \
            -d "{\"tasks\":${tasks_json}}" 2>/dev/null || echo -e "\n000")
        echo "[DEBUG] $(date '+%H:%M:%S') curl completed"

        # Extract HTTP status code (last line) and response body
        http_code=$(echo "$response" | tail -n1)
        response=$(echo "$response" | sed '$d')

        if [[ -z "${response}" ]] || [[ "${http_code}" == "000" ]]; then
            echo " timeout (no response within 10s)"
            # Fall back to individual creation if bulk not available
            # Pass remaining count and starting index
            local remaining=$((total - created))
            local start_index=$((created + 1))
            echo -e "${YELLOW}  Bulk API timeout, falling back to individual creation (remaining: ${remaining})${NC}"
            create_tasks_individual "${remaining}" "${start_index}"
            return
        fi

        if [[ "${http_code}" != "207" ]] && [[ "${http_code}" != "200" ]] && [[ "${http_code}" != "201" ]]; then
            echo " failed (HTTP ${http_code})"
            # Fall back to individual creation on HTTP error
            local remaining=$((total - created))
            local start_index=$((created + 1))
            echo -e "${YELLOW}  Bulk API returned HTTP ${http_code}, falling back to individual creation (remaining: ${remaining})${NC}"
            create_tasks_individual "${remaining}" "${start_index}"
            return
        fi

        # Extract task IDs from bulk response
        # Expected format: {"created":[{"id":"...","title":"..."},...]}
        # or: {"tasks":[{"id":"...","title":"..."},...]}
        local extracted_ids
        extracted_ids=$(echo "${response}" | grep -oE '"id":"[^"]*"' | cut -d'"' -f4 || echo "")

        if [[ -z "${extracted_ids}" ]]; then
            echo " response missing IDs"
            # Bulk response didn't contain IDs, fall back to individual creation
            # Pass remaining count and starting index
            local remaining=$((total - created))
            local start_index=$((created + 1))
            echo -e "${YELLOW}  Bulk response missing IDs, falling back to individual creation (remaining: ${remaining})${NC}"
            create_tasks_individual "${remaining}" "${start_index}"
            return
        fi

        echo " done"
        # Add extracted IDs to TASK_IDS array
        while IFS= read -r task_id; do
            if [[ -n "${task_id}" ]]; then
                TASK_IDS+=("${task_id}")
            fi
        done <<< "${extracted_ids}"

        created=$((created + batch_count))
        local percent=$((created * 100 / total))
        printf "\r  Progress: %d/%d (%d%%)" "${created}" "${total}" "${percent}"
    done
    echo ""
}

# Create tasks individually (fallback)
# @param total: Number of tasks to create
# @param start_index: Starting index for payload generation (default: 1)
create_tasks_individual() {
    local total="$1"
    local start_index="${2:-1}"
    echo "Creating ${total} tasks individually (starting from index ${start_index})..."

    for ((i = 0; i < total; i++)); do
        local task_index=$((start_index + i))
        local payload
        payload=$(generate_task_payload "${task_index}")

        local response
        response=$(curl -sf --connect-timeout 5 --max-time 10 -X POST "${API_URL}/tasks" \
            -H "Content-Type: application/json" \
            -d "${payload}" 2>/dev/null || echo "")

        local task_id
        task_id=$(echo "$response" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
        if [[ -n "$task_id" ]]; then
            TASK_IDS+=("$task_id")
        fi

        local current=$((i + 1))
        if [[ $((current % 100)) -eq 0 ]] || [[ ${current} -eq ${total} ]]; then
            local percent=$((current * 100 / total))
            printf "\r  Progress: %d/%d (%d%%)" "${current}" "${total}" "${percent}"
        fi
    done
    echo ""
}

# Create projects
create_projects() {
    local total="$1"
    echo "Creating ${total} projects..."

    for ((i = 1; i <= total; i++)); do
        local payload
        payload=$(generate_project_payload "$i")

        local response
        response=$(curl -sf --connect-timeout 5 --max-time 10 -X POST "${API_URL}/projects" \
            -H "Content-Type: application/json" \
            -d "${payload}" 2>/dev/null || echo "")

        local project_id
        project_id=$(echo "$response" | grep -o '"project_id":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
        if [[ -n "$project_id" ]]; then
            PROJECT_IDS+=("$project_id")
        fi

        if [[ $((i % 10)) -eq 0 ]] || [[ $i -eq $total ]]; then
            local percent=$((i * 100 / total))
            printf "\r  Progress: %d/%d (%d%%)" "$i" "${total}" "${percent}"
        fi
    done
    echo ""
}

# Create search keyword tasks
create_search_tasks() {
    echo "Creating search keyword tasks..."
    local keywords=("auth" "database" "api" "cache" "test" "benchmark" "performance" "optimization")

    for keyword in "${keywords[@]}"; do
        local payload="{\"title\":\"Implement ${keyword} system\",\"description\":\"Task for ${keyword} functionality\",\"priority\":\"medium\",\"tags\":[\"${keyword}\",\"benchmark\"]}"

        local response
        response=$(curl -sf --connect-timeout 5 --max-time 10 -X POST "${API_URL}/tasks" \
            -H "Content-Type: application/json" \
            -d "${payload}" 2>/dev/null || echo "")

        local task_id
        task_id=$(echo "$response" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "")
        if [[ -n "$task_id" ]]; then
            TASK_IDS+=("$task_id")
            echo "  Created search task: ${task_id} (keyword: ${keyword})"
        fi
    done
}

# =============================================================================
# Main Execution
# =============================================================================

echo "=============================================="
echo "  API Benchmark Data Setup"
echo "=============================================="
echo ""
echo "Configuration:"
echo "  API URL:          ${API_URL}"
echo "  Data scale:       ${DATA_SCALE}"
echo "  Payload variant:  ${PAYLOAD_VARIANT}"
echo "  Batch size:       ${BATCH_SIZE}"
[[ -n "${SEED}" ]] && echo "  Seed:             ${SEED}"
[[ -n "${META_OUTPUT}" ]] && echo "  Meta output:      ${META_OUTPUT}"
echo ""

# Check API health
echo "Checking API health..."
if ! curl -sf --connect-timeout 5 --max-time 10 "${API_URL}/health" > /dev/null 2>&1; then
    echo -e "${RED}ERROR: API is not responding at ${API_URL}/health${NC}"
    exit 1
fi
echo -e "${GREEN}API is healthy${NC}"
echo ""

# Initialize arrays
TASK_IDS=()
PROJECT_IDS=()

# Get target counts
TASK_COUNT="${TASK_COUNTS[$DATA_SCALE]}"
PROJECT_COUNT="${PROJECT_COUNTS[$DATA_SCALE]}"

# Track timing
START_TIME=$(date +%s)

# Create data
create_tasks_bulk "${TASK_COUNT}"
create_search_tasks
create_projects "${PROJECT_COUNT}"

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# Validate that we have at least some IDs
if [[ ${#TASK_IDS[@]} -eq 0 ]]; then
    echo -e "${RED}ERROR: No task IDs were collected. Data creation may have failed.${NC}"
    echo "Check API health and try again."
    exit 1
fi

if [[ ${#PROJECT_IDS[@]} -eq 0 ]]; then
    echo -e "${RED}ERROR: No project IDs were collected. Data creation may have failed.${NC}"
    echo "Check API health and try again."
    exit 1
fi

# Write IDs to Lua file
echo ""
echo "Writing IDs to ${IDS_FILE}..."

cat > "${IDS_FILE}" << EOF
-- Auto-generated test IDs for benchmarking
-- Generated at: $(date)
-- API URL: ${API_URL}
-- Scale: ${DATA_SCALE}
-- Payload: ${PAYLOAD_VARIANT}
$([ -n "${SEED}" ] && echo "-- Seed: ${SEED}")

local M = {}

M.task_ids = {
EOF

# Write up to 1000 task IDs (for Lua table size limits)
local_max_ids=1000
id_count=0
for id in "${TASK_IDS[@]}"; do
    if [[ ${id_count} -ge ${local_max_ids} ]]; then
        break
    fi
    echo "    \"${id}\"," >> "${IDS_FILE}"
    id_count=$((id_count + 1))
done

cat >> "${IDS_FILE}" << EOF
}

M.project_ids = {
EOF

for id in "${PROJECT_IDS[@]}"; do
    echo "    \"${id}\"," >> "${IDS_FILE}"
done

cat >> "${IDS_FILE}" << 'EOF'
}

-- Task states (ID + version pairs) for optimistic locking
-- Initialized with version = 1 for all tasks
M.task_states = {}
for i, id in ipairs(M.task_ids) do
    M.task_states[i] = { id = id, version = 1 }
end

-- Normalize index to valid range
local function normalize_index(index, length)
    if type(index) ~= "number" then
        return nil, "index must be a number"
    end
    if length == 0 then
        return nil, "empty collection"
    end
    return ((index - 1) % length) + 1
end

-- Helper to get a task ID by index (with wrap-around)
function M.get_task_id(index)
    local normalized, err = normalize_index(index, #M.task_ids)
    if err then
        return nil, err
    end
    return M.task_ids[normalized]
end

-- Helper to get a project ID by index (with wrap-around)
function M.get_project_id(index)
    local normalized, err = normalize_index(index, #M.project_ids)
    if err then
        return nil, err
    end
    return M.project_ids[normalized]
end

-- Get task state (ID + version) by index
function M.get_task_state(index)
    local normalized, err = normalize_index(index, #M.task_states)
    if err then
        return nil, err
    end
    local state = M.task_states[normalized]
    return { id = state.id, version = state.version }
end

-- Increment version for a task by index
function M.increment_version(index)
    local normalized, err = normalize_index(index, #M.task_states)
    if err then
        return nil, err
    end
    M.task_states[normalized].version = M.task_states[normalized].version + 1
    return M.task_states[normalized].version
end

-- Set version for a task by index
function M.set_version(index, version)
    local normalized, err = normalize_index(index, #M.task_states)
    if err then
        return false, err
    end
    if type(version) ~= "number" or version < 1 then
        return false, "version must be a positive integer"
    end
    M.task_states[normalized].version = version
    return true
end

-- Reset all versions to 1
function M.reset_versions()
    for i = 1, #M.task_states do
        M.task_states[i].version = 1
    end
end

-- Get total task count
function M.get_task_count()
    return #M.task_ids
end

-- Get total project count
function M.get_project_count()
    return #M.project_ids
end

-- Get all task IDs (copy)
function M.get_all_task_ids()
    local copy = {}
    for i, id in ipairs(M.task_ids) do
        copy[i] = id
    end
    return copy
end

-- Get all project IDs (copy)
function M.get_all_project_ids()
    local copy = {}
    for i, id in ipairs(M.project_ids) do
        copy[i] = id
    end
    return copy
end

return M
EOF

# Output metadata if requested
if [[ -n "${META_OUTPUT}" ]]; then
    echo "Writing metadata to ${META_OUTPUT}..."
    cat > "${META_OUTPUT}" << EOF
{
  "data_scale": "${DATA_SCALE}",
  "payload_variant": "${PAYLOAD_VARIANT}",
  "seed": ${SEED:-null},
  "tasks_created": ${#TASK_IDS[@]},
  "projects_created": ${#PROJECT_IDS[@]},
  "batch_size": ${BATCH_SIZE},
  "duration_seconds": ${DURATION},
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "api_url": "${API_URL}"
}
EOF
fi

echo ""
echo -e "${GREEN}=============================================="
echo "  Setup Complete"
echo "==============================================${NC}"
echo ""
echo "  Tasks created:    ${#TASK_IDS[@]}"
echo "  Projects created: ${#PROJECT_IDS[@]}"
echo "  Duration:         ${DURATION}s"
echo "  IDs file:         ${IDS_FILE}"
[[ -n "${META_OUTPUT}" ]] && echo "  Metadata file:    ${META_OUTPUT}"
echo ""
echo "Run benchmarks with:"
echo "  ./run_benchmark.sh --scenario scenarios/your_scenario.yaml"
