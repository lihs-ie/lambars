#!/bin/bash
# Compare benchmark performance between two git commits using git worktree

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCHMARKS_DIR="$(dirname "${SCRIPT_DIR}")"
API_DIR="$(dirname "${BENCHMARKS_DIR}")"
REPO_ROOT="$(cd "${API_DIR}/../.." && pwd)"

OUTPUT_DIR="${BENCHMARKS_DIR}/results/compare"
PROFILE_MODE=false
QUICK_MODE=false
CLEANUP=false
BEFORE_COMMIT=""
AFTER_COMMIT=""
SCENARIO_FILE=""

WORKTREE_BASE="${REPO_ROOT}/.worktrees"
BEFORE_WORKTREE=""
AFTER_WORKTREE=""

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

show_help() {
    cat <<'EOF'
Usage: compare_commits.sh <before_commit> <after_commit> <scenario_yaml> [options]

Options:
  --profile          Enable perf profiling
  --quick            Quick test (5s duration)
  --output-dir DIR   Output directory (default: benches/results/compare)
  --cleanup          Clean up worktrees after comparison
  --help             Show this help

Example:
  ./compare_commits.sh 407c67a a6ece16 scenarios/tasks_bulk.yaml --profile
EOF
    exit 0
}

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

[[ $# -lt 3 ]] && { echo "Error: Missing required arguments"; echo ""; show_help; }

BEFORE_COMMIT="$1"
AFTER_COMMIT="$2"
SCENARIO_FILE="$3"
shift 3

while [[ $# -gt 0 ]]; do
    case $1 in
        --profile) PROFILE_MODE=true; shift ;;
        --quick) QUICK_MODE=true; shift ;;
        --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
        --cleanup) CLEANUP=true; shift ;;
        --help|-h) show_help ;;
        *) log_error "Unknown option: $1"; show_help ;;
    esac
done

if [[ ! -f "${SCENARIO_FILE}" ]]; then
    for path in "${BENCHMARKS_DIR}/${SCENARIO_FILE}" "${BENCHMARKS_DIR}/scenarios/${SCENARIO_FILE}"; do
        [[ -f "${path}" ]] && { SCENARIO_FILE="${path}"; break; }
    done
    [[ ! -f "${SCENARIO_FILE}" ]] && { log_error "Scenario file not found: ${SCENARIO_FILE}"; exit 1; }
fi

[[ ! -d "${REPO_ROOT}/.git" ]] && { log_error "Not a git repository: ${REPO_ROOT}"; exit 1; }

git -C "${REPO_ROOT}" rev-parse "${BEFORE_COMMIT}" >/dev/null 2>&1 || { log_error "Invalid before commit: ${BEFORE_COMMIT}"; exit 1; }
git -C "${REPO_ROOT}" rev-parse "${AFTER_COMMIT}" >/dev/null 2>&1 || { log_error "Invalid after commit: ${AFTER_COMMIT}"; exit 1; }

create_worktree() {
    local commit="$1"
    local worktree_name="$2"
    local worktree_path="${WORKTREE_BASE}/${worktree_name}"

    log_info "Creating worktree for ${commit} at ${worktree_path}"

    if [[ -d "${worktree_path}" ]]; then
        log_warning "Worktree already exists, removing: ${worktree_path}"
        git -C "${REPO_ROOT}" worktree remove -f "${worktree_path}" 2>/dev/null || true
        rm -rf "${worktree_path}"
    fi

    mkdir -p "${WORKTREE_BASE}"
    git -C "${REPO_ROOT}" worktree add "${worktree_path}" "${commit}" || { log_error "Failed to create worktree for ${commit}"; exit 1; }

    log_success "Worktree created: ${worktree_path}"
    echo "${worktree_path}"
}

cleanup_worktree() {
    local worktree_path="$1"
    [[ -d "${worktree_path}" ]] && { log_info "Cleaning up worktree: ${worktree_path}"; git -C "${REPO_ROOT}" worktree remove -f "${worktree_path}" 2>/dev/null || true; rm -rf "${worktree_path}"; }
}

cleanup_all_worktrees() {
    [[ -n "${BEFORE_WORKTREE}" ]] && cleanup_worktree "${BEFORE_WORKTREE}"
    [[ -n "${AFTER_WORKTREE}" ]] && cleanup_worktree "${AFTER_WORKTREE}"
}

# Only trap cleanup if --cleanup is specified
[[ "${CLEANUP}" == "true" ]] && trap cleanup_all_worktrees EXIT

# Source scenario environment utilities from the current repository
# (not from worktree, as scenario_env.sh may not exist in older commits)
# shellcheck source=scenario_env.sh
source "${SCRIPT_DIR}/scenario_env.sh"

build_api() {
    local worktree_path="$1"
    local api_dir="${worktree_path}/benches/api"

    log_info "Building API in ${worktree_path}"
    cd "${api_dir}"
    cargo build --release || { log_error "Failed to build API in ${worktree_path}"; exit 1; }
    log_success "API built successfully"
}

run_benchmark() {
    local worktree_path="$1"
    local commit="$2"
    local output_subdir="$3"
    local api_dir="${worktree_path}/benches/api"
    local benchmarks_dir="${api_dir}/benchmarks"
    local results_dir="${OUTPUT_DIR}/${output_subdir}"

    log_info "Running benchmark for commit ${commit}"
    mkdir -p "${results_dir}"

    build_api "${worktree_path}"

    # Load and export scenario environment variables before starting API
    # This ensures API server receives the correct configuration
    log_info "Loading scenario environment from ${SCENARIO_FILE}"
    if load_scenario_env "${SCENARIO_FILE}"; then
        export_scenario_env
        log_info "Scenario environment exported: STORAGE_MODE=${STORAGE_MODE:-}, CACHE_MODE=${CACHE_MODE:-}, DATA_SCALE=${DATA_SCALE:-}"
    else
        log_warning "Failed to load scenario environment, using defaults"
    fi

    log_info "Starting API server"
    cd "${api_dir}"
    cargo run --release &
    local api_pid=$!

    log_info "Waiting for API to be ready..."
    local max_wait=30
    for ((waited=0; waited<max_wait; waited++)); do
        curl -sf http://localhost:3002/health >/dev/null 2>&1 && break
        sleep 1
        [[ ${waited} -ge $((max_wait-1)) ]] && { log_error "API failed to start within ${max_wait} seconds"; kill "${api_pid}" 2>/dev/null || true; exit 1; }
    done
    log_success "API is ready"

    cd "${benchmarks_dir}"
    local benchmark_args=("--scenario" "${SCENARIO_FILE}")
    [[ "${QUICK_MODE}" == "true" ]] && benchmark_args+=("--quick")
    [[ "${PROFILE_MODE}" == "true" ]] && benchmark_args+=("--profile")

    log_info "Running: ./run_benchmark.sh ${benchmark_args[*]}"
    ./run_benchmark.sh "${benchmark_args[@]}" || { log_error "Benchmark failed for ${commit}"; kill "${api_pid}" 2>/dev/null || true; exit 1; }
    log_success "Benchmark completed for ${commit}"

    log_info "Stopping API server"
    kill "${api_pid}" 2>/dev/null || true
    wait "${api_pid}" 2>/dev/null || true

    local latest_result
    latest_result=$(find "${benchmarks_dir}/results" -maxdepth 1 -type d -name "20*" | sort -r | head -1)
    [[ -z "${latest_result}" ]] && { log_error "No benchmark results found"; exit 1; }

    log_info "Copying results to ${results_dir}"
    cp -r "${latest_result}"/* "${results_dir}/"
    log_success "Results saved to ${results_dir}"
}

main() {
    cat <<EOF

==============================================
  Git Commit Comparison
==============================================

Before: ${BEFORE_COMMIT}
After:  ${AFTER_COMMIT}
Scenario: ${SCENARIO_FILE}

EOF

    BEFORE_WORKTREE=$(create_worktree "${BEFORE_COMMIT}" "before_${BEFORE_COMMIT}")
    AFTER_WORKTREE=$(create_worktree "${AFTER_COMMIT}" "after_${AFTER_COMMIT}")

    mkdir -p "${OUTPUT_DIR}"
    local timestamp
    timestamp=$(date +%Y%m%d_%H%M%S)
    OUTPUT_DIR="${OUTPUT_DIR}/${timestamp}"
    mkdir -p "${OUTPUT_DIR}"
    log_info "Results will be saved to: ${OUTPUT_DIR}"

    run_benchmark "${BEFORE_WORKTREE}" "${BEFORE_COMMIT}" "before"
    run_benchmark "${AFTER_WORKTREE}" "${AFTER_COMMIT}" "after"

    log_info "Comparing results..."
    cd "${BENCHMARKS_DIR}"
    if ./compare_results.sh "${OUTPUT_DIR}/before" "${OUTPUT_DIR}/after"; then
        log_success "Comparison completed successfully"
    else
        [[ $? -eq 3 ]] && log_error "Regression detected" || log_error "Comparison failed"
    fi

    cat <<EOF

==============================================
  Comparison Complete
==============================================

Results saved to: ${OUTPUT_DIR}

Before: ${OUTPUT_DIR}/before
After:  ${OUTPUT_DIR}/after

EOF

    if [[ "${CLEANUP}" == "true" ]]; then
        log_info "Cleaning up worktrees"
        cleanup_all_worktrees
    else
        log_info "Worktrees preserved (use --cleanup to remove)"
        echo "  Before: ${BEFORE_WORKTREE}"
        echo "  After:  ${AFTER_WORKTREE}"
    fi
}

main "$@"
