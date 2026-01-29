#!/bin/bash
# benches/api/benchmarks/scripts/profile.sh
#
# Profile benchmark with perf and generate flamegraph
#
# Usage:
#   ./profile.sh [options] <scenario.yaml>
#
# Options:
#   --output-dir <dir>   Output directory for results (default: profiling-results/)
#   --duration <secs>    Profile duration (default: 30)
#   --frequency <hz>     Sampling frequency (default: 99)
#   --flamegraph         Generate flamegraph SVG
#   --perf-record        Record perf data only
#   --perf-report        Generate perf report from existing data
#   --help               Show this help message
#
# Environment Variables (can be set via scenario YAML):
#   ENABLE_PERF          Enable perf recording ("1" to enable)
#   ENABLE_FLAMEGRAPH    Enable flamegraph generation ("1" to enable)
#   PERF_FREQUENCY       Sampling frequency in Hz
#   PROFILING_OUTPUT_DIR Output directory for profiling results
#
# Requirements:
#   - perf (Linux) or Instruments (macOS)
#   - FlameGraph tools (https://github.com/brendangregg/FlameGraph)
#   - wrk (for benchmark execution)
#   - yq (for YAML parsing)
#
# Example:
#   ./profile.sh --flamegraph scenarios/profiling_baseline.yaml

set -euo pipefail

# =============================================================================
# Configuration
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCHMARKS_DIR="$(dirname "${SCRIPT_DIR}")"
API_DIR="$(dirname "${BENCHMARKS_DIR}")"

# Default values
OUTPUT_DIR="${PROFILING_OUTPUT_DIR:-profiling-results}"
DURATION="${DURATION:-30}"
FREQUENCY="${PERF_FREQUENCY:-99}"
GENERATE_FLAMEGRAPH="${ENABLE_FLAMEGRAPH:-0}"
RECORD_PERF="${ENABLE_PERF:-0}"
REPORT_ONLY=false
SCENARIO_FILE=""

API_URL="${API_URL:-http://localhost:3002}"
THREADS="${THREADS:-2}"
CONNECTIONS="${CONNECTIONS:-10}"

# FlameGraph repository location (set this to your FlameGraph clone)
FLAMEGRAPH_DIR="${FLAMEGRAPH_DIR:-/usr/local/share/FlameGraph}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# =============================================================================
# Helper Functions
# =============================================================================

show_help() {
    head -35 "$0" | tail -32 | sed 's/^# //' | sed 's/^#//'
    exit 0
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_requirements() {
    local missing=()

    if ! command -v wrk &> /dev/null; then
        missing+=("wrk")
    fi

    if [[ "$GENERATE_FLAMEGRAPH" == "1" ]] || [[ "$RECORD_PERF" == "1" ]]; then
        # Check for perf (Linux) or dtrace (macOS)
        if [[ "$(uname)" == "Linux" ]]; then
            if ! command -v perf &> /dev/null; then
                missing+=("perf (linux-tools-common)")
            fi
        elif [[ "$(uname)" == "Darwin" ]]; then
            # On macOS, we use sample command or Instruments
            if ! command -v sample &> /dev/null; then
                log_warning "sample command not found. CPU profiling may be limited on macOS."
            fi
        fi
    fi

    if [[ "$GENERATE_FLAMEGRAPH" == "1" ]]; then
        if [[ ! -f "${FLAMEGRAPH_DIR}/stackcollapse-perf.pl" ]]; then
            log_warning "FlameGraph tools not found at ${FLAMEGRAPH_DIR}"
            log_warning "Clone from: https://github.com/brendangregg/FlameGraph"
            log_warning "Or set FLAMEGRAPH_DIR environment variable"
        fi
    fi

    if [[ -n "${SCENARIO_FILE}" ]] && ! command -v yq &> /dev/null; then
        missing+=("yq")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing[*]}"
        echo "Install with:"
        echo "  macOS:  brew install ${missing[*]}"
        echo "  Ubuntu: apt-get install ${missing[*]}"
        exit 1
    fi
}

# =============================================================================
# Parse Arguments
# =============================================================================

while [[ $# -gt 0 ]]; do
    case $1 in
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --duration)
            DURATION="$2"
            shift 2
            ;;
        --frequency)
            FREQUENCY="$2"
            shift 2
            ;;
        --flamegraph)
            GENERATE_FLAMEGRAPH="1"
            RECORD_PERF="1"
            shift
            ;;
        --perf-record)
            RECORD_PERF="1"
            shift
            ;;
        --perf-report)
            REPORT_ONLY=true
            shift
            ;;
        --help|-h)
            show_help
            ;;
        *)
            if [[ -f "$1" ]]; then
                SCENARIO_FILE="$1"
            else
                log_error "Unknown option or file not found: $1"
                show_help
            fi
            shift
            ;;
    esac
done

# =============================================================================
# Load Scenario Configuration
# =============================================================================

load_scenario_config() {
    local scenario_file="$1"

    if [[ ! -f "${scenario_file}" ]]; then
        log_error "Scenario file not found: ${scenario_file}"
        exit 1
    fi

    if ! command -v yq &> /dev/null; then
        log_warning "yq not installed. Using default configuration."
        return 0
    fi

    log_info "Loading scenario configuration from: ${scenario_file}"

    # Extract scenario name
    local scenario_name
    scenario_name=$(yq '.name // "benchmark"' "${scenario_file}" | tr -d '"')
    SCENARIO_NAME="${scenario_name}"

    # Extract profiling configuration
    local enable_perf
    enable_perf=$(yq '.profiling.enable_perf // false' "${scenario_file}")
    if [[ "${enable_perf}" == "true" ]]; then
        RECORD_PERF="1"
    fi

    local enable_flamegraph
    enable_flamegraph=$(yq '.profiling.enable_flamegraph // false' "${scenario_file}")
    if [[ "${enable_flamegraph}" == "true" ]]; then
        GENERATE_FLAMEGRAPH="1"
        RECORD_PERF="1"
    fi

    local frequency
    frequency=$(yq '.profiling.frequency // null' "${scenario_file}")
    if [[ "${frequency}" != "null" ]]; then
        FREQUENCY="${frequency}"
    fi

    local output_dir
    output_dir=$(yq '.profiling.output_dir // null' "${scenario_file}" | tr -d '"')
    if [[ "${output_dir}" != "null" && -n "${output_dir}" ]]; then
        OUTPUT_DIR="${output_dir}"
    fi

    # Extract benchmark parameters
    local duration
    duration=$(yq '.duration_seconds // null' "${scenario_file}")
    if [[ "${duration}" != "null" ]]; then
        DURATION="${duration}"
    fi

    local connections
    connections=$(yq '.connections // null' "${scenario_file}")
    if [[ "${connections}" != "null" ]]; then
        CONNECTIONS="${connections}"
    fi

    local threads
    threads=$(yq '.threads // null' "${scenario_file}")
    if [[ "${threads}" != "null" ]]; then
        THREADS="${threads}"
    fi

    log_info "  Scenario name: ${SCENARIO_NAME}"
    log_info "  Duration: ${DURATION}s"
    log_info "  Frequency: ${FREQUENCY}Hz"
    log_info "  Perf recording: ${RECORD_PERF}"
    log_info "  Flamegraph: ${GENERATE_FLAMEGRAPH}"
}

# =============================================================================
# Create Output Directory Structure
# =============================================================================

setup_output_directory() {
    local scenario_name="${SCENARIO_NAME:-benchmark}"
    local timestamp
    timestamp=$(date +%Y%m%d_%H%M%S)

    RESULT_DIR="${BENCHMARKS_DIR}/${OUTPUT_DIR}/${scenario_name}/${timestamp}"
    mkdir -p "${RESULT_DIR}"

    log_info "Output directory: ${RESULT_DIR}"

    # Create metadata file
    cat > "${RESULT_DIR}/metadata.json" << EOF
{
    "scenario_name": "${scenario_name}",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "platform": "$(uname -s)",
    "platform_version": "$(uname -r)",
    "hostname": "$(hostname)",
    "cpu_info": "$(sysctl -n machdep.cpu.brand_string 2>/dev/null || cat /proc/cpuinfo | grep 'model name' | head -1 | cut -d: -f2 | xargs 2>/dev/null || echo 'unknown')",
    "profiling": {
        "frequency_hz": ${FREQUENCY},
        "duration_seconds": ${DURATION},
        "perf_enabled": ${RECORD_PERF},
        "flamegraph_enabled": ${GENERATE_FLAMEGRAPH}
    },
    "benchmark": {
        "threads": ${THREADS},
        "connections": ${CONNECTIONS},
        "api_url": "${API_URL}"
    }
}
EOF
}

# =============================================================================
# Health Check
# =============================================================================

check_api_health() {
    log_info "Checking API health..."

    if curl -sf "${API_URL}/health" > /dev/null 2>&1; then
        log_success "API is responding at ${API_URL}"
        return 0
    else
        log_error "API is not responding at ${API_URL}/health"
        echo ""
        echo "Start the API server with:"
        echo "  cd benches/api && cargo run --release"
        echo "  # or"
        echo "  cd benches/api/docker && docker compose up -d"
        exit 1
    fi
}

# =============================================================================
# Profiling Functions
# =============================================================================

# Get the PID of the running API server
get_api_pid() {
    # Try to find the API process
    local pid
    pid=$(pgrep -f "task-management-benchmark-api" 2>/dev/null | head -1 || true)

    if [[ -z "${pid}" ]]; then
        # Try alternative process names
        pid=$(pgrep -f "target/release/task" 2>/dev/null | head -1 || true)
    fi

    if [[ -z "${pid}" ]]; then
        log_warning "Could not find API server process. Profiling will be skipped."
        return 1
    fi

    echo "${pid}"
}

run_perf_record() {
    local pid="$1"
    local output_file="${RESULT_DIR}/perf.data"

    log_info "Starting perf recording (PID: ${pid}, frequency: ${FREQUENCY}Hz)..."

    if [[ "$(uname)" == "Linux" ]]; then
        # Try with --call-graph dwarf first, fallback to -g (fp) if unsupported
        # Use larger stack size (16KB) for dwarf to handle deep call stacks
        local callgraph_method="--call-graph dwarf,16384"
        local test_file="${RESULT_DIR}/perf_test.data"
        if ! sudo perf record -F "${FREQUENCY}" -p "${pid}" ${callgraph_method} -o "${test_file}" -- sleep 0.5 2>/dev/null; then
            log_warning "--call-graph dwarf not supported, falling back to -g (fp)"
            callgraph_method="-g"
            # Re-validate with -g fallback
            if ! sudo perf record -F "${FREQUENCY}" -p "${pid}" ${callgraph_method} -o "${test_file}" -- sleep 0.5 2>/dev/null; then
                log_error "perf -g also failed. Cannot perform profiling."
                exit 1
            fi
        fi
        # Use sudo rm to handle permission issues with sudo-created files
        sudo rm -f "${test_file}" 2>/dev/null || rm -f "${test_file}" 2>/dev/null

        sudo perf record \
            -F "${FREQUENCY}" \
            -p "${pid}" \
            ${callgraph_method} \
            -o "${output_file}" \
            -- sleep "${DURATION}" &
        PERF_PID=$!
        log_info "  Call-graph method: ${callgraph_method}"
    elif [[ "$(uname)" == "Darwin" ]]; then
        # On macOS, use sample command
        sample "${pid}" "${DURATION}" -f "${output_file}.sample" &
        PERF_PID=$!
    fi
}

stop_perf_record() {
    if [[ -n "${PERF_PID:-}" ]]; then
        log_info "Stopping perf recording..."
        wait "${PERF_PID}" 2>/dev/null || true
    fi
}

generate_perf_report() {
    local perf_data="${RESULT_DIR}/perf.data"
    local report_file="${RESULT_DIR}/perf-report.txt"

    if [[ ! -f "${perf_data}" ]]; then
        log_warning "No perf data found at ${perf_data}"
        return 1
    fi

    log_info "Generating perf report..."

    if [[ "$(uname)" == "Linux" ]]; then
        sudo perf report \
            -i "${perf_data}" \
            --stdio \
            --sort=dso,symbol \
            > "${report_file}" 2>&1
    fi

    if [[ -f "${report_file}" ]]; then
        log_success "Perf report saved to: ${report_file}"
    fi
}

generate_flamegraph() {
    local perf_data="${RESULT_DIR}/perf.data"
    local svg_file="${RESULT_DIR}/flamegraph.svg"
    local collapsed_file="${RESULT_DIR}/collapsed.txt"

    if [[ ! -f "${perf_data}" ]]; then
        log_warning "No perf data found at ${perf_data}"
        return 1
    fi

    if [[ ! -f "${FLAMEGRAPH_DIR}/stackcollapse-perf.pl" ]]; then
        log_error "FlameGraph tools not found at ${FLAMEGRAPH_DIR}"
        log_error "Clone from: https://github.com/brendangregg/FlameGraph"
        return 1
    fi

    log_info "Generating flamegraph..."

    if [[ "$(uname)" == "Linux" ]]; then
        # Convert perf data to collapsed stacks
        sudo perf script -i "${perf_data}" | \
            "${FLAMEGRAPH_DIR}/stackcollapse-perf.pl" > "${collapsed_file}"

        # Generate SVG
        "${FLAMEGRAPH_DIR}/flamegraph.pl" \
            --title "API Benchmark Profile - ${SCENARIO_NAME:-benchmark}" \
            --width 1600 \
            "${collapsed_file}" > "${svg_file}"

        log_success "Flamegraph saved to: ${svg_file}"
    elif [[ "$(uname)" == "Darwin" ]]; then
        local sample_file="${RESULT_DIR}/perf.data.sample"
        if [[ -f "${sample_file}" ]]; then
            # Convert macOS sample output to flamegraph
            "${FLAMEGRAPH_DIR}/stackcollapse-sample.awk" "${sample_file}" > "${collapsed_file}"
            "${FLAMEGRAPH_DIR}/flamegraph.pl" \
                --title "API Benchmark Profile - ${SCENARIO_NAME:-benchmark}" \
                --width 1600 \
                "${collapsed_file}" > "${svg_file}"
            log_success "Flamegraph saved to: ${svg_file}"
        else
            log_warning "No sample data found for macOS flamegraph generation"
        fi
    fi
}

# =============================================================================
# Run Benchmark
# =============================================================================

run_benchmark() {
    local wrk_output="${RESULT_DIR}/wrk-output.json"

    log_info "Running benchmark (duration: ${DURATION}s, threads: ${THREADS}, connections: ${CONNECTIONS})..."

    cd "${BENCHMARKS_DIR}"

    # Extract scenario metadata for environment variables
    local storage_mode="unknown"
    local cache_mode="unknown"
    local load_pattern="unknown"
    local contention_level="unknown"

    if [[ -n "${SCENARIO_FILE}" ]] && command -v yq &> /dev/null; then
        storage_mode=$(yq '.storage_mode // "unknown"' "${SCENARIO_FILE}" | tr -d '"')
        cache_mode=$(yq '.cache_mode // "unknown"' "${SCENARIO_FILE}" | tr -d '"')
        load_pattern=$(yq '.load_pattern // "unknown"' "${SCENARIO_FILE}" | tr -d '"')
        contention_level=$(yq '.contention_level // "unknown"' "${SCENARIO_FILE}" | tr -d '"')
    fi

    # Export scenario metadata as environment variables for result_collector.lua
    export SCENARIO_NAME="${SCENARIO_NAME:-benchmark}"
    export STORAGE_MODE="${storage_mode}"
    export CACHE_MODE="${cache_mode}"
    export LOAD_PATTERN="${load_pattern}"
    export CONTENTION_LEVEL="${contention_level}"
    export OUTPUT_FORMAT="json"
    export THREADS="${THREADS}"
    export CONNECTIONS="${CONNECTIONS}"

    log_info "Scenario metadata exported:"
    log_info "  SCENARIO_NAME=${SCENARIO_NAME}"
    log_info "  STORAGE_MODE=${storage_mode}"
    log_info "  CACHE_MODE=${cache_mode}"
    log_info "  LOAD_PATTERN=${load_pattern}"
    log_info "  CONTENTION_LEVEL=${contention_level}"

    # Run wrk with profile_wrk.lua (dedicated profiling script)
    # profile_wrk.lua provides setup/request/response/done callbacks
    # and integrates with result_collector for extended JSON output
    wrk \
        -t"${THREADS}" \
        -c"${CONNECTIONS}" \
        -d"${DURATION}s" \
        --latency \
        -s scripts/profile_wrk.lua \
        "${API_URL}" 2>&1 | tee "${RESULT_DIR}/wrk-raw-output.txt"

    # Extract JSON from raw output if result_collector produced it
    if [[ -f "${RESULT_DIR}/wrk-raw-output.txt" ]]; then
        # Try to extract JSON block from output (result_collector outputs JSON)
        local json_extracted=false
        if grep -q '^{' "${RESULT_DIR}/wrk-raw-output.txt"; then
            # Extract JSON portion (from first { to last })
            sed -n '/{/,/}/p' "${RESULT_DIR}/wrk-raw-output.txt" | head -1 > "${wrk_output}" 2>/dev/null || true
            if [[ -s "${wrk_output}" ]] && python3 -c "import json; json.load(open('${wrk_output}'))" 2>/dev/null; then
                json_extracted=true
                log_success "Extracted JSON output from result_collector"
            fi
        fi

        # Fallback to manual JSON creation if extraction failed
        if [[ "${json_extracted}" != "true" ]]; then
            log_warning "Could not extract JSON from result_collector output, using fallback parser"
            create_wrk_json_output "${RESULT_DIR}/wrk-raw-output.txt" "${wrk_output}"
        fi
    fi
}

create_wrk_json_output() {
    local raw_output="$1"
    local json_output="$2"

    # Parse wrk output and create JSON
    local reqs_sec
    reqs_sec=$(grep "Requests/sec:" "${raw_output}" | awk '{print $2}' || echo "0")

    local latency_avg
    latency_avg=$(grep "Latency" "${raw_output}" | head -1 | awk '{print $2}' || echo "0")

    local latency_p50
    latency_p50=$(grep "50%" "${raw_output}" | awk '{print $2}' || echo "0")

    local latency_p75
    latency_p75=$(grep "75%" "${raw_output}" | awk '{print $2}' || echo "0")

    local latency_p90
    latency_p90=$(grep "90%" "${raw_output}" | awk '{print $2}' || echo "0")

    local latency_p95
    latency_p95=$(grep "95%" "${raw_output}" 2>/dev/null | awk '{print $2}' || echo "0")

    local latency_p99
    latency_p99=$(grep "99%" "${raw_output}" | awk '{print $2}' || echo "0")

    local total_requests
    total_requests=$(grep "requests in" "${raw_output}" | awk '{print $1}' || echo "0")

    # Extract error counts from wrk output
    local connect_errors read_errors write_errors timeout_errors
    connect_errors=$(grep "Socket errors:" "${raw_output}" | sed 's/.*connect \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
    read_errors=$(grep "Socket errors:" "${raw_output}" | sed 's/.*read \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
    write_errors=$(grep "Socket errors:" "${raw_output}" | sed 's/.*write \([0-9]*\).*/\1/' 2>/dev/null || echo "0")
    timeout_errors=$(grep "Socket errors:" "${raw_output}" | sed 's/.*timeout \([0-9]*\).*/\1/' 2>/dev/null || echo "0")

    # Use environment variables for scenario metadata (exported by run_benchmark)
    local storage_mode="${STORAGE_MODE:-unknown}"
    local cache_mode="${CACHE_MODE:-unknown}"
    local load_pattern="${LOAD_PATTERN:-unknown}"
    local contention_level="${CONTENTION_LEVEL:-unknown}"

    cat > "${json_output}" << EOF
{
    "scenario": {
        "name": "${SCENARIO_NAME:-benchmark}",
        "storage_mode": "${storage_mode}",
        "cache_mode": "${cache_mode}",
        "load_pattern": "${load_pattern}",
        "contention_level": "${contention_level}"
    },
    "execution": {
        "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
        "duration_seconds": ${DURATION},
        "threads": ${THREADS},
        "connections": ${CONNECTIONS}
    },
    "latency": {
        "mean": "${latency_avg}",
        "percentiles": {
            "p50": "${latency_p50}",
            "p75": "${latency_p75}",
            "p90": "${latency_p90}",
            "p95": "${latency_p95}",
            "p99": "${latency_p99}"
        }
    },
    "throughput": {
        "requests_total": ${total_requests},
        "requests_per_second": ${reqs_sec}
    },
    "errors": {
        "connect": ${connect_errors:-0},
        "read": ${read_errors:-0},
        "write": ${write_errors:-0},
        "timeout": ${timeout_errors:-0},
        "status": {
            "4xx": 0,
            "5xx": 0
        }
    },
    "status_distribution": {}
}
EOF

    log_success "Benchmark results saved to: ${json_output}"
}

# =============================================================================
# Report Only Mode (generate report from existing perf.data)
# =============================================================================

generate_report_only() {
    local scenario_file="$1"
    local scenario_name

    if [[ -n "${scenario_file}" ]] && command -v yq &> /dev/null; then
        scenario_name=$(yq '.name // "benchmark"' "${scenario_file}" | tr -d '"')
    else
        scenario_name="${SCENARIO_NAME:-benchmark}"
    fi

    log_info "Generating report from existing perf.data for scenario: ${scenario_name}"

    # Find the latest result directory for this scenario
    local scenario_dir="${BENCHMARKS_DIR}/${OUTPUT_DIR}/${scenario_name}"

    if [[ ! -d "${scenario_dir}" ]]; then
        log_error "No results directory found for scenario: ${scenario_name}"
        log_error "Expected directory: ${scenario_dir}"
        exit 1
    fi

    local latest_dir
    latest_dir=$(find "${scenario_dir}" -maxdepth 1 -type d -name "20*" | sort -r | head -1)

    if [[ -z "${latest_dir}" ]]; then
        log_error "No timestamped result directories found in: ${scenario_dir}"
        exit 1
    fi

    # Check for perf.data
    local perf_data_file="${latest_dir}/perf.data"
    local sample_file="${latest_dir}/perf.data.sample"

    if [[ "$(uname)" == "Linux" ]] && [[ ! -f "${perf_data_file}" ]]; then
        log_error "No perf.data found at: ${perf_data_file}"
        exit 1
    elif [[ "$(uname)" == "Darwin" ]] && [[ ! -f "${sample_file}" ]]; then
        log_error "No perf.data.sample found at: ${sample_file}"
        exit 1
    fi

    log_info "Using result directory: ${latest_dir}"

    # Override RESULT_DIR for report generation functions
    RESULT_DIR="${latest_dir}"

    # Generate perf report
    generate_perf_report || true

    # Generate flamegraph if requested
    if [[ "${GENERATE_FLAMEGRAPH}" == "1" ]]; then
        generate_flamegraph || true
    fi

    log_success "Report generation complete. Results in: ${RESULT_DIR}"
}

# =============================================================================
# Generate Summary
# =============================================================================

generate_summary() {
    local summary_file="${RESULT_DIR}/summary.json"

    log_info "Generating summary..."

    cat > "${summary_file}" << EOF
{
    "scenario_name": "${SCENARIO_NAME:-benchmark}",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "result_directory": "${RESULT_DIR}",
    "files": {
        "metadata": "metadata.json",
        "wrk_output": "wrk-output.json",
        "perf_data": $([ -f "${RESULT_DIR}/perf.data" ] && echo '"perf.data"' || echo 'null'),
        "perf_report": $([ -f "${RESULT_DIR}/perf-report.txt" ] && echo '"perf-report.txt"' || echo 'null'),
        "flamegraph": $([ -f "${RESULT_DIR}/flamegraph.svg" ] && echo '"flamegraph.svg"' || echo 'null')
    },
    "profiling": {
        "perf_enabled": ${RECORD_PERF},
        "flamegraph_enabled": ${GENERATE_FLAMEGRAPH},
        "frequency_hz": ${FREQUENCY}
    }
}
EOF

    log_success "Summary saved to: ${summary_file}"
}

# =============================================================================
# Main
# =============================================================================

main() {
    echo ""
    echo "=============================================="
    echo "  API Benchmark Profiling Tool"
    echo "=============================================="
    echo ""

    # Load scenario configuration if provided
    if [[ -n "${SCENARIO_FILE}" ]]; then
        load_scenario_config "${SCENARIO_FILE}"
    fi

    # Check requirements
    check_requirements

    # Handle --perf-report mode (generate report from existing perf.data only)
    if [[ "${REPORT_ONLY}" == "true" ]]; then
        log_info "Report-only mode: generating report from existing perf.data..."
        generate_report_only "${SCENARIO_FILE}"
        exit 0
    fi

    # Setup output directory
    setup_output_directory

    # Health check
    check_api_health

    # Get API PID for profiling
    local api_pid=""
    if [[ "${RECORD_PERF}" == "1" ]]; then
        api_pid=$(get_api_pid || true)
    fi

    # Start perf recording if enabled and PID found
    if [[ "${RECORD_PERF}" == "1" ]] && [[ -n "${api_pid}" ]]; then
        run_perf_record "${api_pid}"
    fi

    # Run benchmark
    run_benchmark

    # Stop perf recording
    if [[ "${RECORD_PERF}" == "1" ]]; then
        stop_perf_record
    fi

    # Generate perf report if enabled
    if [[ "${RECORD_PERF}" == "1" ]]; then
        generate_perf_report || true
    fi

    # Generate flamegraph if enabled
    if [[ "${GENERATE_FLAMEGRAPH}" == "1" ]]; then
        generate_flamegraph || true
    fi

    # Generate summary
    generate_summary

    echo ""
    echo "=============================================="
    echo "  Profiling Complete"
    echo "=============================================="
    echo ""
    echo "Results saved to: ${RESULT_DIR}"
    echo ""

    # List generated files
    echo "Generated files:"
    ls -la "${RESULT_DIR}"
}

main "$@"
