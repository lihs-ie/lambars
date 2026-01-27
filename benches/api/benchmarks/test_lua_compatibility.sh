#!/usr/bin/env bash
# =============================================================================
# test_lua_compatibility.sh - Lua スクリプト wrk2 互換性テスト
# =============================================================================
# 全ての Lua スクリプトが wrk2 で正常に動作することを確認します。
#
# テスト内容:
#   1. 各 Lua スクリプトを wrk2 で 5 秒間実行
#   2. エラーなく完了することを確認
#   3. Requests/sec 出力があることを確認
#
# 前提条件:
#   - wrk2 がインストールされていること
#   - テスト対象の API サーバーが起動していること (デフォルト: localhost:8080)
#
# 使用方法:
#   ./test_lua_compatibility.sh [--target URL] [--duration SECONDS]
#
# オプション:
#   --target URL       テスト対象の URL (デフォルト: http://localhost:8080)
#   --duration SECONDS 各スクリプトの実行時間 (デフォルト: 5)
#   --skip-server      サーバー起動チェックをスキップ
#   --verbose          詳細出力
# =============================================================================

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPTS_DIR="${SCRIPT_DIR}/scripts"
readonly RESULTS_DIR="${SCRIPT_DIR}/results/lua_compatibility"

# デフォルト設定
DEFAULT_TARGET="http://localhost:8080"
DEFAULT_DURATION=5
DEFAULT_RATE=10

# カラー出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

log_test() {
    echo -e "${BLUE}[TEST]${NC} $*"
}

# =============================================================================
# wrk2 がインストールされているか確認
# =============================================================================
check_wrk2() {
    if ! command -v wrk2 &>/dev/null; then
        log_error "wrk2 is not installed. Please run ./setup_wrk2.sh first."
        exit 1
    fi
    local version
    version=$(wrk2 -v 2>&1 | head -1 || true)
    log_info "Using wrk2: ${version}"
}

# =============================================================================
# サーバーが起動しているか確認
# =============================================================================
check_server() {
    local target="$1"
    local skip_check="$2"

    if [[ "${skip_check}" == "true" ]]; then
        log_warn "Skipping server check"
        return 0
    fi

    log_info "Checking if server is available at ${target}..."

    # ヘルスチェックエンドポイントを試行
    local health_endpoints=("/health" "/api/health" "/" "/ping")

    for endpoint in "${health_endpoints[@]}"; do
        local url="${target}${endpoint}"
        if curl -s -o /dev/null -w "%{http_code}" --connect-timeout 5 "${url}" | grep -q "^[23]"; then
            log_info "Server is available at ${url}"
            return 0
        fi
    done

    log_error "Server is not available at ${target}"
    log_error "Please start the server first:"
    log_error "  cd ../docker && docker compose -f compose.ci.yaml up -d"
    exit 1
}

# =============================================================================
# Lua スクリプトをテスト
# =============================================================================
test_lua_script() {
    local script_path="$1"
    local target="$2"
    local duration="$3"
    local rate="$4"
    local verbose="$5"
    local output_file="$6"

    local script_name
    script_name=$(basename "${script_path}")

    log_test "Testing: ${script_name}"

    # wrk2 実行
    # -t1: 1 スレッド
    # -c1: 1 コネクション
    # -d: 実行時間
    # -R: リクエストレート (requests/sec)
    # -s: Lua スクリプト
    # 注: Lua スクリプトの require() が正しく解決されるよう、scripts ディレクトリで実行
    local wrk2_output
    local exit_code=0

    wrk2_output=$(cd "${SCRIPTS_DIR}" && wrk2 -t1 -c1 -d"${duration}s" -R"${rate}" -s "${script_path}" "${target}" 2>&1) || exit_code=$?

    # 結果をファイルに保存
    echo "=== ${script_name} ===" >> "${output_file}"
    echo "${wrk2_output}" >> "${output_file}"
    echo "" >> "${output_file}"

    # 詳細出力
    if [[ "${verbose}" == "true" ]]; then
        echo "${wrk2_output}"
        echo ""
    fi

    # 結果判定
    local passed=true

    # 1. エラーチェック (非ゼロ終了コード)
    if [[ ${exit_code} -ne 0 ]]; then
        log_error "  FAILED: wrk2 exited with code ${exit_code}"
        passed=false
    fi

    # 2. Lua スクリプトエラーチェック
    if echo "${wrk2_output}" | grep -iq "error"; then
        # "Socket errors" や "Non-2xx or 3xx responses" は警告扱い
        if echo "${wrk2_output}" | grep -iq "lua\|script\|syntax"; then
            log_error "  FAILED: Lua script error detected"
            passed=false
        else
            log_warn "  WARNING: Some errors detected (may be expected)"
        fi
    fi

    # 3. Requests/sec 出力確認
    if ! echo "${wrk2_output}" | grep -q "Requests/sec"; then
        log_error "  FAILED: No 'Requests/sec' in output"
        passed=false
    fi

    # 4. 完了メッセージ確認
    if ! echo "${wrk2_output}" | grep -q "requests in"; then
        log_error "  FAILED: Benchmark did not complete normally"
        passed=false
    fi

    if [[ "${passed}" == "true" ]]; then
        local rps
        rps=$(echo "${wrk2_output}" | grep "Requests/sec" | awk '{print $2}')
        log_info "  PASSED: ${rps} requests/sec"
        return 0
    else
        return 1
    fi
}

# =============================================================================
# メイン処理
# =============================================================================
main() {
    local target="${DEFAULT_TARGET}"
    local duration="${DEFAULT_DURATION}"
    local rate="${DEFAULT_RATE}"
    local skip_server=false
    local verbose=false

    # 引数解析
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --target)
                target="$2"
                shift 2
                ;;
            --duration)
                duration="$2"
                shift 2
                ;;
            --rate)
                rate="$2"
                shift 2
                ;;
            --skip-server)
                skip_server=true
                shift
                ;;
            --verbose|-v)
                verbose=true
                shift
                ;;
            --help|-h)
                echo "Usage: $0 [options]"
                echo ""
                echo "Options:"
                echo "  --target URL       Target URL (default: ${DEFAULT_TARGET})"
                echo "  --duration SECONDS Duration per script (default: ${DEFAULT_DURATION})"
                echo "  --rate RPS         Request rate (default: ${DEFAULT_RATE})"
                echo "  --skip-server      Skip server availability check"
                echo "  --verbose, -v      Verbose output"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    log_info "=== Lua Script wrk2 Compatibility Test ==="
    log_info "Target: ${target}"
    log_info "Duration: ${duration}s per script"
    log_info "Rate: ${rate} req/sec"

    # 前提条件チェック
    check_wrk2
    check_server "${target}" "${skip_server}"

    # 結果ディレクトリ作成
    mkdir -p "${RESULTS_DIR}"
    local timestamp
    timestamp=$(date +%Y%m%d_%H%M%S)
    local output_file="${RESULTS_DIR}/compatibility_test_${timestamp}.txt"

    # ヘッダー情報を記録
    {
        echo "=== Lua Script wrk2 Compatibility Test ==="
        echo "Date: $(date)"
        echo "Target: ${target}"
        echo "Duration: ${duration}s per script"
        echo "Rate: ${rate} req/sec"
        echo "wrk2: $(wrk2 -v 2>&1 | head -1 || true)"
        echo ""
    } > "${output_file}"

    # Lua スクリプト一覧を取得 (macOS/Linux 互換)
    local scripts=()
    while IFS= read -r script; do
        scripts+=("${script}")
    done < <(find "${SCRIPTS_DIR}" -name "*.lua" -type f | sort)

    local total=${#scripts[@]}
    local passed=0
    local failed=0
    local failed_scripts=()

    log_info "Found ${total} Lua scripts to test"
    echo ""

    # 各スクリプトをテスト
    for script in "${scripts[@]}"; do
        if test_lua_script "${script}" "${target}" "${duration}" "${rate}" "${verbose}" "${output_file}"; then
            passed=$((passed + 1))
        else
            failed=$((failed + 1))
            failed_scripts+=("$(basename "${script}")")
        fi
    done

    # サマリー
    echo ""
    log_info "=== Test Summary ==="
    log_info "Total:  ${total}"
    log_info "Passed: ${passed}"

    if [[ ${failed} -gt 0 ]]; then
        log_error "Failed: ${failed}"
        log_error "Failed scripts:"
        for script in "${failed_scripts[@]}"; do
            log_error "  - ${script}"
        done
    else
        log_info "Failed: 0"
    fi

    log_info "Results saved to: ${output_file}"

    # 失敗があれば非ゼロで終了
    if [[ ${failed} -gt 0 ]]; then
        exit 1
    fi

    log_info "=== All Lua scripts are wrk2 compatible ==="
}

main "$@"
