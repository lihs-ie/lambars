#!/usr/bin/env bash
# test_profiling_runner_pin.sh
# profiling.yml の全ジョブが ubuntu-22.04 で固定されていることを検証するテスト

set -euo pipefail

WORKFLOW_FILE="${GITHUB_WORKSPACE:-$(git rev-parse --show-toplevel)}/.github/workflows/profiling.yml"

pass_count=0
fail_count=0

assert_job_pinned() {
    local job="$1"

    # ジョブ名の行から次のジョブ定義開始（2スペースインデントのキー行）までを抽出し runs-on を確認
    local runs_on_value
    runs_on_value=$(awk "
        /^  ${job}:/ { found=1; next }
        found && /^  [a-z]/ { exit }
        found && /runs-on:/ { print; exit }
    " "${WORKFLOW_FILE}" | sed 's/.*runs-on: *//')

    if [[ "${runs_on_value}" == "ubuntu-22.04" ]]; then
        echo "PASS: ジョブ '${job}' が ubuntu-22.04 に固定されている"
        pass_count=$((pass_count + 1))
    elif [[ "${runs_on_value}" == "ubuntu-latest" ]]; then
        echo "FAIL: ジョブ '${job}' が ubuntu-latest を使用している（ubuntu-22.04 が必要）"
        fail_count=$((fail_count + 1))
    else
        echo "FAIL: ジョブ '${job}' の runs-on が見つからないか想定外の値: '${runs_on_value}'"
        fail_count=$((fail_count + 1))
    fi
}

assert_no_ubuntu_latest_anywhere() {
    if grep -q "runs-on: ubuntu-latest" "${WORKFLOW_FILE}"; then
        echo "FAIL: ファイル全体に ubuntu-latest が残存している"
        fail_count=$((fail_count + 1))
    else
        echo "PASS: ファイル全体で ubuntu-latest が使用されていない"
        pass_count=$((pass_count + 1))
    fi
}

echo "=== profiling runner pin テスト ==="
echo "対象ファイル: ${WORKFLOW_FILE}"
echo ""

assert_job_pinned "criterion-profiling-build"
assert_job_pinned "criterion-profiling"
assert_job_pinned "iai-profiling"
assert_job_pinned "api-profiling-matrix"
assert_job_pinned "api-profiling-build"
assert_job_pinned "api-profiling"
assert_job_pinned "summarize-profiling"
assert_no_ubuntu_latest_anywhere

echo ""
echo "=== テスト結果 ==="
echo "PASS: ${pass_count}"
echo "FAIL: ${fail_count}"

if [[ "${fail_count}" -gt 0 ]]; then
    exit 1
fi
echo "全テスト通過ナリ"
