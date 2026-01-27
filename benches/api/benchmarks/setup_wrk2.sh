#!/usr/bin/env bash
# =============================================================================
# setup_wrk2.sh - wrk2 インストールスクリプト
# =============================================================================
# wrk2 (giltene/wrk2) をインストールします。
# wrk2 は wrk のフォークで、正確なレート制御 (-R オプション) をサポートします。
#
# サポート環境:
#   - macOS (Homebrew または ソースビルド)
#   - Linux (ソースビルド)
#
# 使用方法:
#   ./setup_wrk2.sh [--force]
#
# オプション:
#   --force    既にインストール済みでも再インストール
# =============================================================================

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly WRK2_REPO="https://github.com/giltene/wrk2.git"
readonly WRK2_BUILD_DIR="/tmp/wrk2-build"

# カラー出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
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

# =============================================================================
# wrk2 がインストール済みか確認
# =============================================================================
check_wrk2_installed() {
    if command -v wrk2 &>/dev/null; then
        local version
        version=$(wrk2 -v 2>&1 | head -1 || true)
        log_info "wrk2 is already installed: ${version}"
        return 0
    fi
    return 1
}

# =============================================================================
# 必要なビルド依存関係をチェック
# =============================================================================
check_build_dependencies() {
    local missing=()

    # git は必須
    if ! command -v git &>/dev/null; then
        missing+=("git")
    fi

    # make は必須
    if ! command -v make &>/dev/null; then
        missing+=("make")
    fi

    # gcc/clang は必須
    if ! command -v gcc &>/dev/null && ! command -v clang &>/dev/null; then
        missing+=("gcc or clang")
    fi

    # OpenSSL 開発ヘッダのチェック (Linux)
    if [[ "$(uname -s)" == "Linux" ]]; then
        # pkg-config で OpenSSL を確認
        if command -v pkg-config &>/dev/null; then
            if ! pkg-config --exists openssl 2>/dev/null; then
                missing+=("libssl-dev (OpenSSL development headers)")
            fi
        else
            # pkg-config がない場合はヘッダファイルを直接確認
            if [[ ! -f "/usr/include/openssl/ssl.h" ]] && [[ ! -f "/usr/local/include/openssl/ssl.h" ]]; then
                missing+=("libssl-dev (OpenSSL development headers)")
            fi
        fi
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing build dependencies: ${missing[*]}"
        if [[ "$(uname -s)" == "Linux" ]]; then
            log_error "On Debian/Ubuntu: sudo apt-get install -y build-essential libssl-dev git"
            log_error "On RHEL/CentOS: sudo yum install -y gcc make openssl-devel git"
        fi
        return 1
    fi

    return 0
}

# =============================================================================
# macOS: Homebrew でインストール
# =============================================================================
install_macos_homebrew() {
    log_info "Installing wrk2 via Homebrew..."

    # Homebrew がインストールされているか確認
    if ! command -v brew &>/dev/null; then
        log_warn "Homebrew not found. Falling back to source build."
        return 1
    fi

    # wrk2 をインストール (tap 経由)
    if brew tap | grep -q "cfdrake/tap"; then
        brew install cfdrake/tap/wrk2
    else
        brew tap cfdrake/tap
        brew install cfdrake/tap/wrk2
    fi

    # インストール確認
    if command -v wrk2 &>/dev/null; then
        log_info "wrk2 installed successfully via Homebrew"
        return 0
    fi

    log_warn "Homebrew installation failed. Falling back to source build."
    return 1
}

# =============================================================================
# ソースからビルド (macOS/Linux 共通)
# =============================================================================
install_from_source() {
    log_info "Building wrk2 from source..."

    # ビルド依存関係の確認
    if ! check_build_dependencies; then
        log_error "Please install the missing build dependencies first."
        exit 1
    fi

    # 既存のビルドディレクトリを削除
    rm -rf "${WRK2_BUILD_DIR}"

    # リポジトリをクローン
    log_info "Cloning wrk2 repository..."
    git clone --depth 1 "${WRK2_REPO}" "${WRK2_BUILD_DIR}"

    # ビルド
    log_info "Building wrk2..."
    cd "${WRK2_BUILD_DIR}"

    # macOS の場合、OpenSSL パスを設定
    if [[ "$(uname -s)" == "Darwin" ]]; then
        if command -v brew &>/dev/null; then
            local openssl_prefix
            openssl_prefix=$(brew --prefix openssl 2>/dev/null || echo "/usr/local/opt/openssl")
            if [[ -d "${openssl_prefix}" ]]; then
                export LDFLAGS="-L${openssl_prefix}/lib"
                export CPPFLAGS="-I${openssl_prefix}/include"
                log_info "Using OpenSSL from: ${openssl_prefix}"
            fi
        fi
    fi

    # ビルド実行
    local nproc_cmd
    if command -v nproc &>/dev/null; then
        nproc_cmd=$(nproc)
    else
        nproc_cmd=$(sysctl -n hw.ncpu 2>/dev/null || echo 4)
    fi

    make -j"${nproc_cmd}"

    # インストール
    log_info "Installing wrk2 to /usr/local/bin..."
    if [[ -w "/usr/local/bin" ]]; then
        cp wrk /usr/local/bin/wrk2
    else
        sudo cp wrk /usr/local/bin/wrk2
    fi

    # クリーンアップ
    cd "${SCRIPT_DIR}"
    rm -rf "${WRK2_BUILD_DIR}"

    log_info "wrk2 built and installed successfully from source"
}

# =============================================================================
# インストール検証
# =============================================================================
verify_installation() {
    log_info "Verifying wrk2 installation..."

    if ! command -v wrk2 &>/dev/null; then
        log_error "wrk2 command not found in PATH"
        return 1
    fi

    local version
    version=$(wrk2 -v 2>&1 | head -1 || true)
    log_info "wrk2 version: ${version}"

    # 簡易動作確認 (ヘルプ表示)
    if ! wrk2 --help &>/dev/null; then
        log_error "wrk2 --help failed"
        return 1
    fi

    log_info "wrk2 installation verified successfully"
    return 0
}

# =============================================================================
# メイン処理
# =============================================================================
main() {
    local force=false

    # 引数解析
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --force)
                force=true
                shift
                ;;
            --help|-h)
                echo "Usage: $0 [--force]"
                echo ""
                echo "Options:"
                echo "  --force    Reinstall even if wrk2 is already installed"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    log_info "=== wrk2 Installation Script ==="

    # 既にインストール済みの場合はスキップ (--force でない場合)
    if [[ "${force}" == "false" ]] && check_wrk2_installed; then
        log_info "wrk2 is already installed. Use --force to reinstall."
        exit 0
    fi

    # OS に応じたインストール方法を選択
    local os_type
    os_type=$(uname -s)

    case "${os_type}" in
        Darwin)
            log_info "Detected macOS"
            # Homebrew を試し、失敗したらソースビルド
            if ! install_macos_homebrew; then
                install_from_source
            fi
            ;;
        Linux)
            log_info "Detected Linux"
            # Linux はソースビルドのみ
            install_from_source
            ;;
        *)
            log_error "Unsupported OS: ${os_type}"
            exit 1
            ;;
    esac

    # インストール検証
    if ! verify_installation; then
        log_error "wrk2 installation verification failed"
        exit 1
    fi

    log_info "=== wrk2 installation complete ==="
}

main "$@"
