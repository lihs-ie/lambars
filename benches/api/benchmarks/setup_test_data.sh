#!/bin/bash
# benches/api/benchmarks/setup_test_data.sh
#
# Create test data for benchmark and save IDs to a file
#
# Usage:
#   ./setup_test_data.sh                    # Setup with default API URL
#   API_URL=http://localhost:3001 ./setup_test_data.sh  # Custom URL

set -euo pipefail

API_URL="${API_URL:-http://localhost:3002}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IDS_FILE="${SCRIPT_DIR}/scripts/test_ids.lua"

echo "Setting up test data at ${API_URL}..."
echo ""

# Check API health
if ! curl -sf "${API_URL}/health" > /dev/null 2>&1; then
    echo "ERROR: API is not responding at ${API_URL}/health"
    exit 1
fi

# Create tasks and collect IDs
echo "Creating test tasks..."
TASK_IDS=()

for i in {1..10}; do
    RESPONSE=$(curl -sf -X POST "${API_URL}/tasks" \
        -H "Content-Type: application/json" \
        -d "{\"title\":\"Benchmark Task ${i}\",\"description\":\"Test task for benchmarking\",\"priority\":\"medium\",\"tags\":[\"benchmark\",\"test\"]}")

    TASK_ID=$(echo "$RESPONSE" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
    if [[ -n "$TASK_ID" ]]; then
        TASK_IDS+=("$TASK_ID")
        echo "  Created task: $TASK_ID"
    fi
done

# Create tasks with search keywords for phase2_alternative benchmark
echo "Creating search keyword tasks..."
SEARCH_KEYWORDS=("auth" "database" "api" "cache" "test")
for keyword in "${SEARCH_KEYWORDS[@]}"; do
    RESPONSE=$(curl -sf -X POST "${API_URL}/tasks" \
        -H "Content-Type: application/json" \
        -d "{\"title\":\"Implement ${keyword} system\",\"description\":\"Task for ${keyword} functionality\",\"priority\":\"medium\",\"tags\":[\"${keyword}\",\"benchmark\"]}")

    TASK_ID=$(echo "$RESPONSE" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
    if [[ -n "$TASK_ID" ]]; then
        TASK_IDS+=("$TASK_ID")
        echo "  Created search task: $TASK_ID (keyword: ${keyword})"
    fi
done

# Create projects and collect IDs
echo "Creating test projects..."
PROJECT_IDS=()

for i in {1..3}; do
    RESPONSE=$(curl -sf -X POST "${API_URL}/projects" \
        -H "Content-Type: application/json" \
        -d "{\"name\":\"Benchmark Project ${i}\",\"description\":\"Test project for benchmarking\"}")

    PROJECT_ID=$(echo "$RESPONSE" | grep -o '"project_id":"[^"]*"' | head -1 | cut -d'"' -f4)
    if [[ -n "$PROJECT_ID" ]]; then
        PROJECT_IDS+=("$PROJECT_ID")
        echo "  Created project: $PROJECT_ID"
    fi
done

# Write IDs to Lua file
echo ""
echo "Writing IDs to ${IDS_FILE}..."

cat > "${IDS_FILE}" << EOF
-- Auto-generated test IDs for benchmarking
-- Generated at: $(date)
-- API URL: ${API_URL}

local M = {}

M.task_ids = {
EOF

for id in "${TASK_IDS[@]}"; do
    echo "    \"${id}\"," >> "${IDS_FILE}"
done

cat >> "${IDS_FILE}" << EOF
}

M.project_ids = {
EOF

for id in "${PROJECT_IDS[@]}"; do
    echo "    \"${id}\"," >> "${IDS_FILE}"
done

cat >> "${IDS_FILE}" << EOF
}

-- Helper to get a task ID by index (with wrap-around)
function M.get_task_id(index)
    return M.task_ids[((index - 1) % #M.task_ids) + 1]
end

-- Helper to get a project ID by index (with wrap-around)
function M.get_project_id(index)
    return M.project_ids[((index - 1) % #M.project_ids) + 1]
end

return M
EOF

echo ""
echo "Setup complete!"
echo "  Tasks created: ${#TASK_IDS[@]}"
echo "  Projects created: ${#PROJECT_IDS[@]}"
echo "  IDs file: ${IDS_FILE}"
echo ""
echo "Run benchmarks with:"
echo "  ./run_benchmark.sh"
