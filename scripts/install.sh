#!/usr/bin/env bash
set -euo pipefail

REPO="EffortlessMetrics/perl-lsp"
BIN_NAME="perllsp"
DAP_BIN_NAME="perl-dap"
VERSION="${VERSION:-latest}"
PREFER_GNU="${PREFER_GNU:-0}"
PERL_LSP_LINUX_LIBC="${PERL_LSP_LINUX_LIBC:-auto}"
PRINT_TARGET=0

if [ "${1:-}" = "--print-target" ]; then
    PRINT_TARGET=1
    shift
fi

if [ -z "${INSTALL_DIR:-}" ]; then
    if [ -w /usr/local/bin ] 2>/dev/null; then
        INSTALL_DIR="/usr/local/bin"
    else
        INSTALL_DIR="$HOME/.local/bin"
    fi
fi

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
say() { printf '%b\n' "$1"; }
info() { say "${GREEN}=>${NC} $1"; }
warn() { say "${YELLOW}warning:${NC} $1" >&2; }
err() { say "${RED}error:${NC} $1" >&2; exit 1; }
need_cmd() { command -v "$1" >/dev/null 2>&1 || err "required command not found: $1"; }

normalize_linux_libc_choice() {
    case "${1:-auto}" in
        auto|"") echo "auto" ;;
        glibc|gnu) echo "gnu" ;;
        musl) echo "musl" ;;
        *) err "invalid PERL_LSP_LINUX_LIBC value: $1
Expected one of: auto, glibc, gnu, musl" ;;
    esac
}

detect_linux_libc() {
    local choice
    choice="auto"
    case "${PERL_LSP_LINUX_LIBC:-auto}" in
        auto|"") choice="auto" ;;
        glibc|gnu) choice="gnu" ;;
        musl) choice="musl" ;;
        *) err "invalid PERL_LSP_LINUX_LIBC value: ${PERL_LSP_LINUX_LIBC:-}
Expected one of: auto, glibc, gnu, musl" ;;
    esac
    if [ "$choice" != "auto" ]; then echo "$choice"; return; fi
    if [ "$PREFER_GNU" = "1" ]; then
        warn "PREFER_GNU=1 is deprecated; prefer PERL_LSP_LINUX_LIBC=glibc"
        echo "gnu"; return
    fi
    if [ -f /etc/alpine-release ]; then echo "musl"; return; fi
    if command -v ldd >/dev/null 2>&1 && ldd --version 2>&1 | grep -qi musl; then echo "musl"; return; fi
    if ls /lib/ld-musl-*.so.1 /usr/lib/ld-musl-*.so.1 >/dev/null 2>&1; then echo "musl"; return; fi
    echo "gnu"
}

describe_target() {
    case "$1" in
      x86_64-unknown-linux-gnu) echo "Linux x64 / AMD64 using glibc (most distributions)" ;;
      aarch64-unknown-linux-gnu) echo "Linux ARM64 using glibc (most distributions)" ;;
      x86_64-unknown-linux-musl) echo "Linux x64 / AMD64 using musl (Alpine/musl)" ;;
      aarch64-unknown-linux-musl) echo "Linux ARM64 using musl (Alpine/musl)" ;;
      x86_64-apple-darwin) echo "macOS Intel" ;;
      aarch64-apple-darwin) echo "macOS Apple Silicon" ;;
      *) echo "$1" ;;
    esac
}

detect_platform() {
    local os arch termux libc
    os="$(uname -s)"; arch="$(uname -m)"; termux=0
    if [ -n "${TERMUX_VERSION:-}" ] || [ -d "/data/data/com.termux/files/usr/bin" ]; then termux=1; fi
    case "$arch" in x86_64|amd64|x64) arch=x86_64;; aarch64|arm64) arch=aarch64;; *) err "unsupported architecture: $arch";; esac
    case "$os" in
      Linux)
        if [ "$termux" = "1" ]; then
          err "Termux/Android does not currently have a pre-built release asset.

Install from source instead:
  pkg install rust
  cargo install perllsp

Then configure your editor to run:
  perllsp --stdio"
        fi
        libc="$(detect_linux_libc)"
        TARGET="${arch}-unknown-linux-${libc}"
        ;;
      Darwin) TARGET="${arch}-apple-darwin" ;;
      *) err "unsupported operating system: $os" ;;
    esac
    info "detected: $(describe_target "$TARGET")"
    info "selected release asset target: $TARGET"
}

resolve_version() {
    if [ "$VERSION" = "latest" ]; then
        local api="https://api.github.com/repos/${REPO}/releases/latest" json
        json="$(curl -fsSL "$api")" || err "failed to query GitHub API"
        TAG="$(printf '%s' "$json" | grep '"tag_name"' | sed -E 's/.*"tag_name": ?"([^"]+)".*/\1/')"
        [ -n "$TAG" ] || err "could not parse tag_name"
    else
        case "$VERSION" in v*) TAG="$VERSION";; *) TAG="v$VERSION";; esac
    fi
    VERSION_NUM="${TAG#v}"
    info "version: $TAG"
}

download_and_verify() {
    local asset="${BIN_NAME}-${VERSION_NUM}-${TARGET}.tar.gz"
    local base="https://github.com/${REPO}/releases/download/${TAG}"
    local archive="${TMPDIR}/${asset}" sums="${TMPDIR}/SHA256SUMS"
    info "downloading ${asset}"
    curl -fsSL --progress-bar "${base}/${asset}" -o "$archive" || err "download failed for ${asset}"
    curl -fsSL "${base}/SHA256SUMS" -o "$sums" || err "SHA256SUMS is required but could not be downloaded"
    local expected actual
    expected="$(grep "${asset}" "$sums" | awk '{print $1}')"
    [ -n "$expected" ] || err "checksum entry not found for ${asset}"
    if command -v sha256sum >/dev/null 2>&1; then actual="$(sha256sum "$archive" | awk '{print $1}')"; else actual="$(shasum -a 256 "$archive" | awk '{print $1}')"; fi
    [ "$expected" = "$actual" ] || err "checksum mismatch for ${asset}"
    ARCHIVE_PATH="$archive"; EXTRACT_DIR="${TMPDIR}/${BIN_NAME}-${VERSION_NUM}-${TARGET}"
}

main() {
    say ""; say "Perl LSP installer"; say "=================="; say ""
    need_cmd curl; need_cmd tar
    detect_platform
    if [ "$PRINT_TARGET" = "1" ]; then printf '%s\n' "$TARGET"; exit 0; fi
    resolve_version
    TMPDIR="$(mktemp -d)"; trap 'rm -rf "$TMPDIR"' EXIT
    download_and_verify
    tar -xzf "$ARCHIVE_PATH" -C "$TMPDIR"
    mkdir -p "$INSTALL_DIR"
    cp "${EXTRACT_DIR}/${BIN_NAME}" "$INSTALL_DIR/${BIN_NAME}" && chmod 755 "$INSTALL_DIR/${BIN_NAME}"
    if [ -f "${EXTRACT_DIR}/${DAP_BIN_NAME}" ]; then cp "${EXTRACT_DIR}/${DAP_BIN_NAME}" "$INSTALL_DIR/${DAP_BIN_NAME}" && chmod 755 "$INSTALL_DIR/${DAP_BIN_NAME}"; fi
    info "installed: $INSTALL_DIR/$BIN_NAME"
}

main "$@"
