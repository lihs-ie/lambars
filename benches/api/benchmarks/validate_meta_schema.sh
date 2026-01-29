#!/bin/bash
# benches/api/benchmarks/validate_meta_schema.sh
#
# Validate meta.json and meta_extended.json files against JSON Schema
#
# Usage:
#   ./validate_meta_schema.sh <meta.json>              # Validate single file
#   ./validate_meta_schema.sh <dir>                    # Validate all meta.json in directory
#   ./validate_meta_schema.sh --all <results_dir>     # Validate all meta.json recursively
#   ./validate_meta_schema.sh --extended <dir>        # Also validate meta_extended.json files
#
# Exit codes:
#   0 - All validations passed
#   1 - Validation failed or error

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCHEMA_FILE="${SCRIPT_DIR}/schema/meta_v3.json"
SCHEMA_EXTENDED_FILE="${SCRIPT_DIR}/schema/meta_extended.schema.json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Validation results
TOTAL=0
PASSED=0
FAILED=0

# Extended validation flag
VALIDATE_EXTENDED=false

print_usage() {
    echo "Usage: $0 <meta.json|dir> [--all] [--extended]"
    echo ""
    echo "Options:"
    echo "  <meta.json>    Validate a single meta.json file"
    echo "  <dir>          Validate all meta.json files in directory"
    echo "  --all <dir>    Validate all meta.json files recursively"
    echo "  --extended     Also validate meta_extended.json files (if present)"
    echo ""
    echo "Examples:"
    echo "  $0 results/scenario_1/meta.json"
    echo "  $0 results/"
    echo "  $0 --all results/"
    echo "  $0 --all --extended results/"
}

# Check if required tools are available
check_tools() {
    # Try ajv-cli first (faster, more complete)
    if command -v ajv &> /dev/null; then
        echo "ajv"
        return 0
    fi

    # Try Python jsonschema
    if command -v python3 &> /dev/null && python3 -c "import jsonschema" 2>/dev/null; then
        echo "python"
        return 0
    fi

    # Try jq for basic structure validation
    if command -v jq &> /dev/null; then
        echo "jq"
        return 0
    fi

    echo "none"
    return 1
}

# Validate using ajv-cli
validate_with_ajv() {
    local file="$1"
    local schema="${2:-${SCHEMA_FILE}}"
    ajv validate -s "${schema}" -d "${file}" --spec=draft7 2>&1
}

# Validate using Python jsonschema
validate_with_python() {
    local file="$1"
    local schema="${2:-${SCHEMA_FILE}}"
    python3 << EOF
import json
import sys
try:
    from jsonschema import validate, ValidationError, Draft7Validator
except ImportError:
    print("jsonschema module not found")
    sys.exit(1)

try:
    with open("${schema}") as f:
        schema_data = json.load(f)
    with open("${file}") as f:
        data = json.load(f)

    validator = Draft7Validator(schema_data)
    errors = list(validator.iter_errors(data))

    if errors:
        for error in errors:
            path = ".".join(str(p) for p in error.absolute_path) or "(root)"
            print(f"  - {path}: {error.message}")
        sys.exit(1)
    else:
        print("valid")
        sys.exit(0)
except json.JSONDecodeError as e:
    print(f"JSON parse error: {e}")
    sys.exit(1)
except FileNotFoundError as e:
    print(f"File not found: {e}")
    sys.exit(1)
EOF
}

# Validate using jq (basic structure check only)
validate_with_jq() {
    local file="$1"
    local errors=()

    # Check JSON validity
    if ! jq -e . "${file}" &>/dev/null; then
        echo "Invalid JSON"
        return 1
    fi

    # Check required top-level fields using has() (null values are valid)
    local required_fields=("version" "scenario" "execution" "results" "errors")
    for field in "${required_fields[@]}"; do
        if ! jq -e "has(\"${field}\")" "${file}" &>/dev/null; then
            errors+=("Missing required field: ${field}")
        fi
    done

    # Check version
    local version
    version=$(jq -r '.version // "missing"' "${file}")
    if [[ "${version}" != "3.0" ]]; then
        errors+=("Invalid version: ${version} (expected 3.0)")
    fi

    # Check required results fields using has() (some allow null values)
    local results_fields=("requests" "duration_seconds" "error_rate" "latency_ms" "http_status" "retries")
    for field in "${results_fields[@]}"; do
        if ! jq -e ".results | has(\"${field}\")" "${file}" &>/dev/null; then
            errors+=("Missing results.${field}")
        fi
    done

    # Check required latency_ms fields using has() (p50/p90/p99 allow null)
    local latency_fields=("p50" "p90" "p99")
    for field in "${latency_fields[@]}"; do
        if ! jq -e ".results.latency_ms | has(\"${field}\")" "${file}" &>/dev/null; then
            errors+=("Missing results.latency_ms.${field}")
        fi
    done

    # Check error_rate is number or null (not string)
    local error_rate_type
    error_rate_type=$(jq -r '.results.error_rate | type' "${file}")
    if [[ "${error_rate_type}" != "number" && "${error_rate_type}" != "null" ]]; then
        errors+=("error_rate must be number or null, got ${error_rate_type}")
    fi

    # Check latency values are positive numbers or null (exclusiveMinimum: 0)
    for field in p50 p90 p99; do
        local value_type value
        value_type=$(jq -r ".results.latency_ms.${field} | type" "${file}")
        # Skip null values
        if [[ "${value_type}" == "null" ]]; then
            continue
        fi
        # Check type is number
        if [[ "${value_type}" != "number" ]]; then
            errors+=("results.latency_ms.${field} must be number or null, got ${value_type}")
            continue
        fi
        # Check value is positive (exclusiveMinimum: 0)
        value=$(jq -r ".results.latency_ms.${field}" "${file}")
        if awk -v val="${value}" 'BEGIN { exit (val + 0 <= 0) ? 0 : 1 }'; then
            errors+=("results.latency_ms.${field} must be positive (exclusiveMinimum: 0)")
        fi
    done

    if [[ ${#errors[@]} -gt 0 ]]; then
        for error in "${errors[@]}"; do
            echo "  - ${error}"
        done
        return 1
    fi

    echo "valid (jq basic check)"
    return 0
}

# Validate meta_extended.json using jq (basic structure check only)
validate_extended_with_jq() {
    local file="$1"
    local errors=()

    # Check JSON validity
    if ! jq -e . "${file}" &>/dev/null; then
        echo "Invalid JSON"
        return 1
    fi

    # Check required top-level fields
    local required_fields=("version" "rate_control" "integration" "phases")
    for field in "${required_fields[@]}"; do
        if ! jq -e "has(\"${field}\")" "${file}" &>/dev/null; then
            errors+=("Missing required field: ${field}")
        fi
    done

    # Check version
    local version
    version=$(jq -r '.version // "missing"' "${file}")
    if [[ "${version}" != "1.0" ]]; then
        errors+=("Invalid version: ${version} (expected 1.0)")
    fi

    # Check rate_control required fields
    local rate_control_fields=("wrk_version" "rate_control_enabled")
    for field in "${rate_control_fields[@]}"; do
        if ! jq -e ".rate_control | has(\"${field}\")" "${file}" &>/dev/null; then
            errors+=("Missing rate_control.${field}")
        fi
    done

    # Type check: rate_control.rate_control_enabled must be boolean
    local enabled_type
    enabled_type=$(jq -r '.rate_control.rate_control_enabled | type' "${file}" 2>/dev/null)
    if [[ "${enabled_type}" != "boolean" ]]; then
        errors+=("rate_control.rate_control_enabled must be boolean, got ${enabled_type}")
    fi

    # Type check: rate_control.target_rps must be number or null
    local target_type
    target_type=$(jq -r '.rate_control.target_rps | type' "${file}" 2>/dev/null)
    if [[ "${target_type}" != "number" && "${target_type}" != "null" ]]; then
        errors+=("rate_control.target_rps must be number or null, got ${target_type}")
    fi

    # Type check: rate_control.actual_rps must be number or null
    local actual_type
    actual_type=$(jq -r '.rate_control.actual_rps | type' "${file}" 2>/dev/null)
    if [[ "${actual_type}" != "number" && "${actual_type}" != "null" ]]; then
        errors+=("rate_control.actual_rps must be number or null, got ${actual_type}")
    fi

    # Type check: rate_control.rps_within_tolerance must be boolean or null
    local tolerance_type
    tolerance_type=$(jq -r '.rate_control.rps_within_tolerance | type' "${file}" 2>/dev/null)
    if [[ "${tolerance_type}" != "boolean" && "${tolerance_type}" != "null" ]]; then
        errors+=("rate_control.rps_within_tolerance must be boolean or null, got ${tolerance_type}")
    fi

    # Check integration required fields
    local integration_fields=("rps_method" "latency_p99_method" "error_rate_method")
    for field in "${integration_fields[@]}"; do
        if ! jq -e ".integration | has(\"${field}\")" "${file}" &>/dev/null; then
            errors+=("Missing integration.${field}")
        fi
    done

    # Check phases is an array
    if ! jq -e '.phases | type == "array"' "${file}" &>/dev/null; then
        errors+=("phases must be an array")
    else
        # Check each phase has required fields and correct types
        local phase_count
        phase_count=$(jq '.phases | length' "${file}" 2>/dev/null || echo "0")
        for ((i=0; i<phase_count; i++)); do
            local phase_fields=("phase" "target_rps" "actual_rps" "duration_seconds")
            for field in "${phase_fields[@]}"; do
                if ! jq -e ".phases[${i}] | has(\"${field}\")" "${file}" &>/dev/null; then
                    errors+=("Missing phases[${i}].${field}")
                fi
            done

            # Type check: phases[i].phase must be string
            local phase_name_type
            phase_name_type=$(jq -r ".phases[${i}].phase | type" "${file}" 2>/dev/null)
            if [[ "${phase_name_type}" != "string" ]]; then
                errors+=("phases[${i}].phase must be string, got ${phase_name_type}")
            fi

            # Type check: phases[i].target_rps must be number
            local phase_target_type
            phase_target_type=$(jq -r ".phases[${i}].target_rps | type" "${file}" 2>/dev/null)
            if [[ "${phase_target_type}" != "number" ]]; then
                errors+=("phases[${i}].target_rps must be number, got ${phase_target_type}")
            fi

            # Type check: phases[i].actual_rps must be number
            local phase_actual_type
            phase_actual_type=$(jq -r ".phases[${i}].actual_rps | type" "${file}" 2>/dev/null)
            if [[ "${phase_actual_type}" != "number" ]]; then
                errors+=("phases[${i}].actual_rps must be number, got ${phase_actual_type}")
            fi

            # Type check: phases[i].duration_seconds must be number
            local phase_duration_type
            phase_duration_type=$(jq -r ".phases[${i}].duration_seconds | type" "${file}" 2>/dev/null)
            if [[ "${phase_duration_type}" != "number" ]]; then
                errors+=("phases[${i}].duration_seconds must be number, got ${phase_duration_type}")
            fi
        done
    fi

    if [[ ${#errors[@]} -gt 0 ]]; then
        for error in "${errors[@]}"; do
            echo "  - ${error}"
        done
        return 1
    fi

    echo "valid (jq basic check)"
    return 0
}

# Validate a single file
validate_file() {
    local file="$1"
    local tool="$2"
    local schema="${3:-${SCHEMA_FILE}}"
    local result
    local exit_code

    TOTAL=$((TOTAL + 1))

    if [[ ! -f "${file}" ]]; then
        echo -e "${RED}FAIL${NC} ${file}: File not found"
        FAILED=$((FAILED + 1))
        return 1
    fi

    # Disable set -e temporarily to capture exit code
    set +e
    case "${tool}" in
        ajv)
            result=$(validate_with_ajv "${file}" "${schema}" 2>&1)
            exit_code=$?
            ;;
        python)
            result=$(validate_with_python "${file}" "${schema}" 2>&1)
            exit_code=$?
            ;;
        jq)
            result=$(validate_with_jq "${file}" 2>&1)
            exit_code=$?
            ;;
        *)
            echo -e "${RED}FAIL${NC} ${file}: No validation tool available"
            FAILED=$((FAILED + 1))
            set -e
            return 1
            ;;
    esac
    set -e

    if [[ ${exit_code} -eq 0 ]]; then
        echo -e "${GREEN}PASS${NC} ${file}"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}FAIL${NC} ${file}"
        echo "${result}" | sed 's/^/  /'
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Validate a single meta_extended.json file
validate_extended_file() {
    local file="$1"
    local tool="$2"
    local result
    local exit_code

    TOTAL=$((TOTAL + 1))

    if [[ ! -f "${file}" ]]; then
        # meta_extended.json is optional, skip silently if not present
        TOTAL=$((TOTAL - 1))
        return 0
    fi

    # Disable set -e temporarily to capture exit code
    set +e
    case "${tool}" in
        ajv)
            result=$(validate_with_ajv "${file}" "${SCHEMA_EXTENDED_FILE}" 2>&1)
            exit_code=$?
            ;;
        python)
            result=$(validate_with_python "${file}" "${SCHEMA_EXTENDED_FILE}" 2>&1)
            exit_code=$?
            ;;
        jq)
            result=$(validate_extended_with_jq "${file}" 2>&1)
            exit_code=$?
            ;;
        *)
            echo -e "${RED}FAIL${NC} ${file}: No validation tool available"
            FAILED=$((FAILED + 1))
            set -e
            return 1
            ;;
    esac
    set -e

    if [[ ${exit_code} -eq 0 ]]; then
        echo -e "${GREEN}PASS${NC} ${file}"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}FAIL${NC} ${file}"
        echo "${result}" | sed 's/^/  /'
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Main
main() {
    if [[ $# -lt 1 ]]; then
        print_usage
        exit 1
    fi

    # Check schema file exists
    if [[ ! -f "${SCHEMA_FILE}" ]]; then
        echo -e "${RED}Error: Schema file not found: ${SCHEMA_FILE}${NC}"
        exit 1
    fi

    # Detect validation tool
    local tool
    tool=$(check_tools) || {
        echo -e "${RED}Error: No validation tool available.${NC}"
        echo "Install one of: ajv-cli (npm), jsonschema (pip), or jq"
        exit 1
    }
    echo -e "${YELLOW}Using validation tool: ${tool}${NC}"
    echo ""

    local recursive=false
    local target=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --all)
                recursive=true
                shift
                ;;
            --extended)
                VALIDATE_EXTENDED=true
                shift
                ;;
            -h|--help)
                print_usage
                exit 0
                ;;
            *)
                target="$1"
                shift
                ;;
        esac
    done

    if [[ -z "${target}" ]]; then
        print_usage
        exit 1
    fi

    # Check extended schema file exists if --extended is used
    if [[ "${VALIDATE_EXTENDED}" == "true" && ! -f "${SCHEMA_EXTENDED_FILE}" ]]; then
        echo -e "${RED}Error: Extended schema file not found: ${SCHEMA_EXTENDED_FILE}${NC}"
        exit 1
    fi

    # Validate
    if [[ -f "${target}" ]]; then
        # Single file
        if [[ "$(basename "${target}")" == "meta_extended.json" ]]; then
            validate_extended_file "${target}" "${tool}"
        else
            validate_file "${target}" "${tool}"
        fi
    elif [[ -d "${target}" ]]; then
        # Directory - validate meta.json files
        local find_opts=(-name "meta.json")
        if [[ "${recursive}" == "false" ]]; then
            find_opts+=(-maxdepth 1)
        fi

        while IFS= read -r -d '' file; do
            validate_file "${file}" "${tool}" || true
        done < <(find "${target}" "${find_opts[@]}" -print0 2>/dev/null)

        # If --extended, also validate meta_extended.json files
        if [[ "${VALIDATE_EXTENDED}" == "true" ]]; then
            echo ""
            echo -e "${YELLOW}Validating meta_extended.json files...${NC}"
            local find_extended_opts=(-name "meta_extended.json")
            if [[ "${recursive}" == "false" ]]; then
                find_extended_opts+=(-maxdepth 1)
            fi

            while IFS= read -r -d '' file; do
                validate_extended_file "${file}" "${tool}" || true
            done < <(find "${target}" "${find_extended_opts[@]}" -print0 2>/dev/null)
        fi
    else
        echo -e "${RED}Error: ${target} is not a file or directory${NC}"
        exit 1
    fi

    # Summary
    echo ""
    echo "========================================="
    echo "Validation Summary"
    echo "========================================="
    echo "Total:  ${TOTAL}"
    echo -e "Passed: ${GREEN}${PASSED}${NC}"
    echo -e "Failed: ${RED}${FAILED}${NC}"

    # Fail if no files were validated (likely generation failure)
    if [[ ${TOTAL} -eq 0 ]]; then
        echo -e "${RED}Error: No meta.json files found to validate${NC}"
        exit 1
    fi

    if [[ ${FAILED} -gt 0 ]]; then
        exit 1
    fi
    exit 0
}

main "$@"
