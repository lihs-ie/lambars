#!/usr/bin/env bash
# validate_threshold_feasibility.sh
#
# Preflight lint: Detect theoretically infeasible RPS thresholds before running benchmarks.
#
# Usage:
#   ./validate_threshold_feasibility.sh --scenario-file <path> --threshold-file <path> [--mode strict|warn]
#
# Arguments:
#   --scenario-file   Path to scenario YAML file
#   --threshold-file  Path to thresholds.yaml
#   --mode            strict (default) = exit 1 on infeasible; warn = exit 0 with warning
#
# Output:
#   PASS: <scenario_name> (<metric>: upper_bound=<N>, threshold=<N>)
#   FAIL: <scenario_name> (<metric>: upper_bound=<N>, threshold=<N>) [INFEASIBLE]
#
# Exit codes:
#   0  All checks passed (or mode=warn)
#   1  Infeasible threshold found (mode=strict only)

set -euo pipefail

# =============================================================================
# Argument parsing
# =============================================================================

SCENARIO_FILE=""
THRESHOLD_FILE=""
MODE="strict"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --scenario-file)
            SCENARIO_FILE="$2"
            shift 2
            ;;
        --threshold-file)
            THRESHOLD_FILE="$2"
            shift 2
            ;;
        --mode)
            MODE="$2"
            shift 2
            ;;
        *)
            echo "ERROR: Unknown argument: $1" >&2
            exit 1
            ;;
    esac
done

die() { echo "ERROR: $*" >&2; exit 1; }

[[ -z "${SCENARIO_FILE}" ]]  && die "--scenario-file is required"
[[ -z "${THRESHOLD_FILE}" ]] && die "--threshold-file is required"
[[ ! -f "${SCENARIO_FILE}" ]]  && die "Scenario file not found: ${SCENARIO_FILE}"
[[ ! -f "${THRESHOLD_FILE}" ]] && die "Threshold file not found: ${THRESHOLD_FILE}"
[[ "${MODE}" != "strict" && "${MODE}" != "warn" ]] && die "--mode must be 'strict' or 'warn'"

# =============================================================================
# Read scenario fields
# =============================================================================

SCENARIO_NAME=$(yq '.name // ""' "${SCENARIO_FILE}" | tr -d '"')
PROFILE_TYPE=$(yq '.rps_profile // "steady"' "${SCENARIO_FILE}" | tr -d '"')
TARGET_RPS=$(yq '.target_rps // 0' "${SCENARIO_FILE}" | tr -d '"')
MIN_RPS=$(yq '.min_rps // 0' "${SCENARIO_FILE}" | tr -d '"')
STEP_COUNT=$(yq '.step_count // 0' "${SCENARIO_FILE}" | tr -d '"')
DURATION_SECONDS=$(yq '.duration_seconds // 0' "${SCENARIO_FILE}" | tr -d '"')
BURST_MULTIPLIER=$(yq '.burst_multiplier // 1.0' "${SCENARIO_FILE}" | tr -d '"')
BURST_DURATION=$(yq '.burst_duration_seconds // 0' "${SCENARIO_FILE}" | tr -d '"')
BURST_INTERVAL=$(yq '.burst_interval_seconds // 0' "${SCENARIO_FILE}" | tr -d '"')

if [[ -z "${SCENARIO_NAME}" ]]; then
    echo "ERROR: Scenario file missing 'name' field: ${SCENARIO_FILE}" >&2
    exit 1
fi

# =============================================================================
# Read RPS threshold rule from thresholds.yaml
# =============================================================================

RPS_METRIC=$(SCENARIO_NAME="${SCENARIO_NAME}" \
    yq '.scenarios[env(SCENARIO_NAME)].rps.metric // ""' "${THRESHOLD_FILE}" 2>/dev/null | tr -d '"')

if [[ -z "${RPS_METRIC}" || "${RPS_METRIC}" == "null" ]]; then
    echo "PASS: ${SCENARIO_NAME} (no rps rule defined - SKIP)"
    exit 0
fi

RPS_ERROR_THRESHOLD=$(SCENARIO_NAME="${SCENARIO_NAME}" \
    yq '.scenarios[env(SCENARIO_NAME)].rps.error // 0' "${THRESHOLD_FILE}" 2>/dev/null | tr -d '"')

if [[ -z "${RPS_ERROR_THRESHOLD}" || "${RPS_ERROR_THRESHOLD}" == "0" ]]; then
    echo "PASS: ${SCENARIO_NAME} (no rps.error threshold defined - SKIP)"
    exit 0
fi

# =============================================================================
# Calculate theoretical upper bound per profile type
# =============================================================================

UPPER_BOUND="0"

case "${PROFILE_TYPE}" in
    steady|constant)
        UPPER_BOUND="${TARGET_RPS}"
        ;;

    burst)
        case "${RPS_METRIC}" in
            peak_phase_rps)
                UPPER_BOUND="${TARGET_RPS}"
                ;;
            weighted_rps)
                # weighted_rps = burst_ratio * target_rps + (1 - burst_ratio) * base_rps
                # where base_rps = target_rps / burst_multiplier
                if [[ "${BURST_INTERVAL}" == "0" || "${BURST_MULTIPLIER}" == "0" ]]; then
                    UPPER_BOUND="${TARGET_RPS}"
                else
                    UPPER_BOUND=$(awk \
                        -v target="${TARGET_RPS}" \
                        -v multiplier="${BURST_MULTIPLIER}" \
                        -v burst_duration="${BURST_DURATION}" \
                        -v burst_interval="${BURST_INTERVAL}" \
                        'BEGIN {
                            burst_ratio = burst_duration / burst_interval
                            base_rps = target / multiplier
                            print burst_ratio * target + (1 - burst_ratio) * base_rps
                        }')
                fi
                ;;
            *)
                UPPER_BOUND="${TARGET_RPS}"
                ;;
        esac
        ;;

    step_up)
        # Equal steps from min_rps to target_rps; weighted average is the upper bound
        case "${RPS_METRIC}" in
            weighted_rps)
                if [[ "${STEP_COUNT}" -le 1 || "${DURATION_SECONDS}" == "0" ]]; then
                    UPPER_BOUND="${TARGET_RPS}"
                else
                    UPPER_BOUND=$(awk \
                        -v target="${TARGET_RPS}" \
                        -v min_rps="${MIN_RPS}" \
                        -v step_count="${STEP_COUNT}" \
                        -v duration="${DURATION_SECONDS}" \
                        'BEGIN {
                            step_duration = duration / step_count
                            total_rps_time = 0
                            for (i = 1; i <= step_count; i++) {
                                step_rps = min_rps + (target - min_rps) * i / step_count
                                total_rps_time += step_rps * step_duration
                            }
                            print total_rps_time / duration
                        }')
                fi
                ;;
            *)
                UPPER_BOUND="${TARGET_RPS}"
                ;;
        esac
        ;;

    ramp_up_down)
        case "${RPS_METRIC}" in
            sustain_phase_rps)
                UPPER_BOUND="${TARGET_RPS}"
                ;;
            weighted_rps)
                # Ramp up/down phases reduce average to approximately 75% of target
                UPPER_BOUND=$(awk -v target="${TARGET_RPS}" 'BEGIN { print target * 0.75 }')
                ;;
            *)
                UPPER_BOUND="${TARGET_RPS}"
                ;;
        esac
        ;;

    *)
        UPPER_BOUND="${TARGET_RPS}"
        ;;
esac

# =============================================================================
# Feasibility check
# =============================================================================

UPPER_BOUND_ROUNDED=$(awk -v n="${UPPER_BOUND}" 'BEGIN { printf "%.0f", n }')

if awk -v upper_bound="${UPPER_BOUND}" -v threshold="${RPS_ERROR_THRESHOLD}" \
    'BEGIN { exit (upper_bound >= threshold) ? 0 : 1 }'; then
    echo "PASS: ${SCENARIO_NAME} (${RPS_METRIC}: upper_bound=${UPPER_BOUND_ROUNDED}, threshold=${RPS_ERROR_THRESHOLD})"
    exit 0
else
    echo "FAIL: ${SCENARIO_NAME} (${RPS_METRIC}: upper_bound=${UPPER_BOUND_ROUNDED}, threshold=${RPS_ERROR_THRESHOLD}) [INFEASIBLE]"
    [[ "${MODE}" == "strict" ]] && exit 1
    exit 0
fi
