#!/bin/bash
# Extract hot functions from perf profiling data

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

TOP_N=10
FILTER_PATTERN=""
JSON_OUTPUT=false
THRESHOLD=0.1
OUTPUT_FILE=""
PERF_DATA=""

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

show_help() {
    cat <<'EOF'
Usage: extract_hot_functions.sh <perf_data_file> [options]

Options:
  --top N            Show top N functions (default: 10)
  --filter PATTERN   Filter functions by pattern (grep regex)
  --json             Output as JSON
  --threshold NUM    Only show functions above threshold % (default: 0.1)
  --output FILE      Save output to file
  --help             Show this help

Example:
  ./extract_hot_functions.sh perf.data --top 20 --filter "lambars|persistent"
EOF
    exit 0
}

log_info() { echo -e "${BLUE}[INFO]${NC} $1" >&2; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1" >&2; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1" >&2; }
log_error() { echo -e "${RED}[ERROR]${NC} $1" >&2; }

[[ $# -eq 0 || "$1" == "--help" || "$1" == "-h" ]] && { [[ $# -eq 0 ]] && echo "Error: Missing perf data file" >&2 && echo "" >&2; show_help; }

PERF_DATA="$1"
shift

while [[ $# -gt 0 ]]; do
    case $1 in
        --top) TOP_N="$2"; shift 2 ;;
        --filter) FILTER_PATTERN="$2"; shift 2 ;;
        --json) JSON_OUTPUT=true; shift ;;
        --threshold) THRESHOLD="$2"; shift 2 ;;
        --output) OUTPUT_FILE="$2"; shift 2 ;;
        --help|-h) show_help ;;
        *) log_error "Unknown option: $1"; show_help ;;
    esac
done

[[ ! -f "${PERF_DATA}" ]] && { log_error "Perf data file not found: ${PERF_DATA}"; exit 1; }
[[ "$(uname)" != "Linux" ]] && { log_error "This script requires Linux with perf support"; exit 1; }
command -v perf &> /dev/null || { log_error "perf command not found"; echo "Install with: apt-get install linux-tools-common linux-tools-generic" >&2; exit 1; }
[[ "${JSON_OUTPUT}" == "true" ]] && ! command -v jq &> /dev/null && log_warning "jq not found, JSON output may be malformed"

# Check if running as root or if perf can read the data file without sudo
if [[ $EUID -ne 0 ]]; then
    if ! perf report -i "${PERF_DATA}" --stdio -n 2>/dev/null | head -1 > /dev/null; then
        log_warning "perf report requires root privileges. Attempting with sudo..."
        PERF_CMD="sudo perf"
    else
        PERF_CMD="perf"
    fi
else
    PERF_CMD="perf"
fi

extract_hot_functions() {
    local perf_data="$1"
    log_info "Extracting hot functions from ${perf_data}"

    local report_output
    report_output=$(${PERF_CMD} report -i "${perf_data}" --stdio --sort=symbol,dso --no-children -n 2>/dev/null || true)
    [[ -z "${report_output}" ]] && { log_error "Failed to generate perf report"; exit 1; }

    local output=""
    local line_count=0

    while IFS= read -r line; do
        [[ "${line}" =~ ^# ]] && continue

        local overhead samples dso symbol
        overhead=$(echo "${line}" | awk '{print $1}' | sed 's/%//')

        [[ -n "${overhead}" ]] && (( $(echo "${overhead} < ${THRESHOLD}" | bc -l 2>/dev/null || echo "0") )) && continue

        samples=$(echo "${line}" | awk '{print $2}')
        dso=$(echo "${line}" | awk '{print $3}')
        symbol=$(echo "${line}" | awk '{$1=$2=$3=""; print $0}' | sed 's/^ *//' | sed 's/\[.\] //')

        [[ -n "${FILTER_PATTERN}" ]] && ! echo "${symbol}" | grep -qE -- "${FILTER_PATTERN}" && continue
        [[ -z "${overhead}" || -z "${symbol}" ]] && continue

        if [[ "${JSON_OUTPUT}" == "true" ]]; then
            local escaped_symbol escaped_dso
            # Properly escape for JSON: quotes, backslashes, newlines, tabs
            escaped_symbol=$(echo "${symbol}" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g' | tr '\n' ' ')
            escaped_dso=$(echo "${dso}" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g' | tr '\n' ' ')
            output+="{\"overhead_percent\": ${overhead}, \"samples\": ${samples}, \"dso\": \"${escaped_dso}\", \"symbol\": \"${escaped_symbol}\"},"$'\n'
        else
            output+=$(printf "%-8s | %-10s | %-30s | %s\n" "${overhead}%" "${samples}" "${dso}" "${symbol}")$'\n'
        fi

        line_count=$((line_count + 1))
        [[ ${line_count} -ge ${TOP_N} ]] && break
    done <<< "${report_output}"

    if [[ "${JSON_OUTPUT}" == "true" ]]; then
        output=$(echo "${output}" | sed '$ s/,$//')
        echo "[${output}]"
    else
        local header separator
        header=$(printf "%-8s | %-10s | %-30s | %s\n" "Overhead" "Samples" "DSO" "Symbol")
        separator=$(printf "%-8s-+-%-10s-+-%-30s-+--%s\n" "--------" "----------" "------------------------------" "------")
        echo "${header}"$'\n'"${separator}"$'\n'"${output}"
    fi
}

main() {
    if [[ "${JSON_OUTPUT}" != "true" ]]; then
        cat >&2 <<EOF

==============================================
  Hot Function Extraction
==============================================

EOF
        log_info "Perf data: ${PERF_DATA}"
        log_info "Top N: ${TOP_N}"
        log_info "Threshold: ${THRESHOLD}%"
        [[ -n "${FILTER_PATTERN}" ]] && log_info "Filter: ${FILTER_PATTERN}"
        echo "" >&2
    fi

    local result
    result=$(extract_hot_functions "${PERF_DATA}")

    if [[ -n "${OUTPUT_FILE}" ]]; then
        echo "${result}" > "${OUTPUT_FILE}"
        log_success "Results saved to: ${OUTPUT_FILE}"
    else
        echo "${result}"
    fi

    [[ "${JSON_OUTPUT}" != "true" ]] && echo -e "\n==============================================\n" >&2
}

main "$@"
