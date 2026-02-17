#!/usr/bin/env bash
# Validates profiling artifacts (stacks.folded and flamegraph.svg) for integrity.
# Usage:
#   Single directory: ./validate_profiling_artifacts.sh <artifact_dir>
#   All artifacts:    ./validate_profiling_artifacts.sh --all <root_dir> [--report <file>]
# Exit code: 0 = all pass, 1 = violations found

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

ALL_MODE=false
ALL_DIR=""
REPORT_FILE=""
SINGLE_DIR=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --all)
            ALL_MODE=true
            ALL_DIR="$2"
            shift 2
            ;;
        --report)
            REPORT_FILE="$2"
            shift 2
            ;;
        *)
            SINGLE_DIR="$1"
            shift
            ;;
    esac
done

declare -a ARTIFACT_DIRS=()

if [[ "${ALL_MODE}" == "true" ]]; then
    if [[ -z "${ALL_DIR}" || ! -d "${ALL_DIR}" ]]; then
        echo -e "${RED}ERROR: --all requires a valid directory${NC}" >&2
        exit 1
    fi
    # Collect directories containing stacks.folded or flamegraph.svg (deduplicated)
    while IFS= read -r -d '' artifact_file; do
        dir="$(dirname "${artifact_file}")"
        already_added=false
        for existing_dir in "${ARTIFACT_DIRS[@]+"${ARTIFACT_DIRS[@]}"}"; do
            if [[ "${existing_dir}" == "${dir}" ]]; then
                already_added=true
                break
            fi
        done
        if [[ "${already_added}" == "false" ]]; then
            ARTIFACT_DIRS+=("${dir}")
        fi
    done < <(find "${ALL_DIR}" -type f \( -name 'stacks.folded' -o -name 'flamegraph.svg' \) -print0 2>/dev/null)

    if [[ ${#ARTIFACT_DIRS[@]} -eq 0 ]]; then
        echo -e "${RED}ERROR: No profiling artifacts found in ${ALL_DIR}${NC}" >&2
        exit 1
    fi
else
    if [[ -z "${SINGLE_DIR}" ]]; then
        echo -e "${RED}ERROR: artifact_dir is required${NC}" >&2
        echo "Usage: $0 <artifact_dir>" >&2
        echo "       $0 --all <root_dir> [--report <file>]" >&2
        exit 1
    fi
    if [[ ! -d "${SINGLE_DIR}" ]]; then
        echo -e "${RED}ERROR: directory not found: ${SINGLE_DIR}${NC}" >&2
        exit 1
    fi
    ARTIFACT_DIRS+=("${SINGLE_DIR}")
fi

PASS_COUNT=0
FAIL_COUNT=0
REPORT=""
declare -a VIOLATIONS=()

add_report() { REPORT+="$1"$'\n'; }

# Checks stacks.folded in the given directory.
# Appends any violations to the provided array variable name (nameref).
check_stacks_folded() {
    local directory="$1"
    local -n check_violations=$2
    local stacks_file="${directory}/stacks.folded"

    [[ -f "${stacks_file}" ]] || return 0

    if grep -qF "perf not found" "${stacks_file}" 2>/dev/null; then
        check_violations+=("stacks.folded: contains 'perf not found' error")
    fi
    if grep -qF "assertion failed" "${stacks_file}" 2>/dev/null; then
        check_violations+=("stacks.folded: contains 'assertion failed' error")
    fi
    if ! grep -qE '^[^[:space:]].+[[:space:]]+[0-9]+$' "${stacks_file}" 2>/dev/null; then
        check_violations+=("stacks.folded: no valid stack count lines found (expected '<stack> <count>' format)")
    fi
}

# Checks flamegraph.svg in the given directory.
# Appends any violations to the provided array variable name (nameref).
check_flamegraph_svg() {
    local directory="$1"
    local -n check_violations=$2
    local svg_file="${directory}/flamegraph.svg"

    [[ -f "${svg_file}" ]] || return 0

    if grep -qF "No valid input provided to flamegraph" "${svg_file}" 2>/dev/null; then
        check_violations+=("flamegraph.svg: contains 'No valid input provided to flamegraph'")
    fi
    if grep -qF "No stack counts found" "${svg_file}" 2>/dev/null; then
        check_violations+=("flamegraph.svg: contains 'No stack counts found'")
    fi
    if grep -qE 'ERROR:' "${svg_file}" 2>/dev/null; then
        check_violations+=("flamegraph.svg: contains 'ERROR:' prefix")
    fi
}

validate_artifact_directory() {
    local directory="$1"
    local dir_name
    dir_name="$(basename "${directory}")"
    local violations_for_dir=()

    check_stacks_folded "${directory}" violations_for_dir
    check_flamegraph_svg "${directory}" violations_for_dir

    if [[ ${#violations_for_dir[@]} -eq 0 ]]; then
        echo -e "${GREEN}PASS: ${dir_name}${NC}" >&2
        add_report "PASS: ${dir_name}"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo -e "${RED}FAIL: ${dir_name}${NC}" >&2
        add_report "FAIL: ${dir_name}"
        for violation in "${violations_for_dir[@]}"; do
            echo -e "  ${YELLOW}> ${violation}${NC}" >&2
            add_report "  > ${violation}"
            VIOLATIONS+=("${dir_name}: ${violation}")
        done
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
}

add_report "=== Profiling Artifact Integrity Report ==="
add_report "Date: $(date)"
add_report "Directories: ${#ARTIFACT_DIRS[@]}"
add_report ""

for artifact_dir in "${ARTIFACT_DIRS[@]}"; do
    validate_artifact_directory "${artifact_dir}"
done

add_report ""
add_report "=== Summary ==="
add_report "Total: ${#ARTIFACT_DIRS[@]}"
add_report "Pass: ${PASS_COUNT}"
add_report "Fail: ${FAIL_COUNT}"

if [[ -n "${REPORT_FILE}" ]]; then
    mkdir -p "$(dirname "${REPORT_FILE}")"
    printf '%s\n' "${REPORT}" > "${REPORT_FILE}"
    echo "Report written to ${REPORT_FILE}" >&2
fi

if [[ ${#VIOLATIONS[@]} -gt 0 ]]; then
    echo "" >&2
    echo -e "${RED}=== Violations Summary ===${NC}" >&2
    for violation in "${VIOLATIONS[@]}"; do
        echo -e "${RED}  ${violation}${NC}" >&2
    done
    echo "" >&2
    echo -e "${RED}Total violations: ${#VIOLATIONS[@]}${NC}" >&2
    exit 1
else
    echo "" >&2
    echo -e "${GREEN}All profiling artifacts passed (${PASS_COUNT}/${#ARTIFACT_DIRS[@]})${NC}" >&2
    exit 0
fi
