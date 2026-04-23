#!/usr/bin/env bash
# Perl LSP installer — Linux and macOS
#
# Usage (curl-pipeable):
#   curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/scripts/install.sh | bash
#
# Or with options via environment variables:
#   VERSION=v0.12.0 INSTALL_DIR=/usr/local/bin bash scripts/install.sh
#   PREFER_GNU=1 bash scripts/install.sh   # prefer glibc over musl on Linux
#
# Supported platforms:
#   Linux x86_64 (musl/gnu), Linux aarch64 (musl/gnu), macOS x86_64, macOS aarch64
set -euo pipefail

REPO="EffortlessMetrics/perl-lsp"
BIN_NAME="perllsp"
DAP_BIN_NAME="perl-dap"
VERSION="${VERSION:-latest}"
PREFER_GNU="${PREFER_GNU:-0}"

# Determine install directory: user-local by default, system-wide if explicitly set
if [ -z "${INSTALL_DIR:-}" ]; then
    if [ -n "${TERMUX_VERSION:-}" ] || [ -d "/data/data/com.termux/files/usr/bin" ]; then
        INSTALL_DIR="/data/data/com.termux/files/usr/bin"
    elif [ -w /usr/local/bin ] 2>/dev/null; then
        INSTALL_DIR="/usr/local/bin"
    else
        INSTALL_DIR="$HOME/.local/bin"
    fi
fi

# ── Output helpers ─────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

say()     { printf '%b\n' "$1"; }
info()    { say "${GREEN}=>${NC} $1"; }
warn()    { say "${YELLOW}warning:${NC} $1" >&2; }
err()     { say "${RED}error:${NC} $1" >&2; exit 1; }

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "required command not found: $1"
    fi
}

# ── Platform detection ─────────────────────────────────────────────────────────

detect_platform() {
    local _os _arch _libc _termux

    _os="$(uname -s)"
    _arch="$(uname -m)"
    _termux=0

    if [ -n "${TERMUX_VERSION:-}" ] || [ -d "/data/data/com.termux/files/usr/bin" ]; then
        _termux=1
    fi

    case "$_os" in
        Linux)
            _os="linux"
            # Termux uses Android's bionic libc. Prefer static musl binaries there.
            # For other Linux environments, prefer musl unless caller overrides.
            if [ "$_termux" = "1" ]; then
                _libc="musl"
            elif [ "$PREFER_GNU" = "1" ]; then
                _libc="gnu"
            else
                _libc="musl"
            fi
            ;;
        Darwin)
            _os="darwin"
            _libc=""
            ;;
        MINGW*|MSYS*|CYGWIN*)
            err "Windows is not supported by this script. Use the PowerShell installer instead:
  irm https://raw.githubusercontent.com/$REPO/master/scripts/install.ps1 | iex"
            ;;
        *)
            err "unsupported operating system: $_os"
            ;;
    esac

    case "$_arch" in
        x86_64|amd64)   _arch="x86_64" ;;
        aarch64|arm64)  _arch="aarch64" ;;
        *)              err "unsupported architecture: $_arch" ;;
    esac

    if [ "$_os" = "linux" ]; then
        TARGET="${_arch}-unknown-linux-${_libc}"
    else
        TARGET="${_arch}-apple-darwin"
    fi

    info "platform: $_os $_arch (target: $TARGET)"
    if [ "$_termux" = "1" ]; then
        info "termux environment detected; using musl release artifacts for compatibility"
    fi
}

# ── Version resolution ─────────────────────────────────────────────────────────

resolve_version() {
    if [ "$VERSION" = "latest" ]; then
        info "fetching latest release..."
        local _api_url="https://api.github.com/repos/${REPO}/releases/latest"
        local _json

        if ! _json="$(curl -fsSL "$_api_url" 2>/dev/null)"; then
            err "failed to query GitHub API: ${_api_url}
Check your internet connection or set VERSION=v<x.y.z> to pin a version."
        fi

        TAG="$(printf '%s' "$_json" | grep '"tag_name"' | sed -E 's/.*"tag_name": ?"([^"]+)".*/\1/')"
        if [ -z "$TAG" ]; then
            err "could not parse tag_name from GitHub API response"
        fi
    else
        # Accept "0.12.0" or "v0.12.0"
        case "$VERSION" in
            v*) TAG="$VERSION" ;;
            *)  TAG="v$VERSION" ;;
        esac
    fi

    # Strip leading 'v' to form the version number used in asset names.
    VERSION_NUM="${TAG#v}"
    info "version: $TAG"
}

# ── Download and verify ────────────────────────────────────────────────────────

download_and_verify() {
    local _asset="${BIN_NAME}-${VERSION_NUM}-${TARGET}.tar.gz"
    local _base_url="https://github.com/${REPO}/releases/download/${TAG}"

    ASSET_URL="${_base_url}/${_asset}"
    CHECKSUM_URL="${_base_url}/SHA256SUMS"

    TMPDIR="$(mktemp -d)"
    # shellcheck disable=SC2064
    trap "rm -rf '$TMPDIR'" EXIT

    local _archive="${TMPDIR}/${_asset}"

    info "downloading ${_asset}"
    if ! curl -fsSL --progress-bar "$ASSET_URL" -o "$_archive"; then
        err "download failed: $ASSET_URL

If this version does not have a pre-built binary for your platform, try:
  cargo install perllsp
  # or
  cargo install perllsp --target $TARGET"
    fi

    # Verify checksum when SHA256SUMS is available.
    local _sums="${TMPDIR}/SHA256SUMS"
    if curl -fsSL "$CHECKSUM_URL" -o "$_sums" 2>/dev/null; then
        local _expected _actual
        _expected="$(grep "${_asset}" "$_sums" | awk '{print $1}')"

        if [ -z "$_expected" ]; then
            warn "no checksum entry found for ${_asset}; skipping verification"
        else
            if command -v sha256sum >/dev/null 2>&1; then
                _actual="$(sha256sum "$_archive" | awk '{print $1}')"
            elif command -v shasum >/dev/null 2>&1; then
                _actual="$(shasum -a 256 "$_archive" | awk '{print $1}')"
            else
                warn "neither sha256sum nor shasum found; skipping checksum verification"
                _actual=""
            fi

            if [ -n "$_actual" ]; then
                if [ "$_expected" = "$_actual" ]; then
                    info "checksum verified"
                else
                    err "checksum mismatch for ${_asset}
  expected: $_expected
  actual:   $_actual
The download may be corrupted. Delete any cached files and retry."
                fi
            fi
        fi
    else
        warn "could not download SHA256SUMS; skipping checksum verification"
    fi

    ARCHIVE_PATH="$_archive"
    EXTRACT_DIR="${TMPDIR}/${BIN_NAME}-${VERSION_NUM}-${TARGET}"
}

# ── Extract ────────────────────────────────────────────────────────────────────

extract_archive() {
    info "extracting archive"
    tar -xzf "$ARCHIVE_PATH" -C "$TMPDIR"

    if [ ! -d "$EXTRACT_DIR" ]; then
        err "expected directory not found after extraction: $EXTRACT_DIR
The release archive may have an unexpected layout."
    fi
}

# ── Install ────────────────────────────────────────────────────────────────────

install_binaries() {
    local _src_bin="${EXTRACT_DIR}/${BIN_NAME}"
    if [ ! -f "$_src_bin" ]; then
        err "binary not found in archive: $_src_bin"
    fi

    mkdir -p "$INSTALL_DIR"

    # Verify we can write to the install directory.
    if [ ! -w "$INSTALL_DIR" ]; then
        err "install directory is not writable: $INSTALL_DIR
Try one of:
  sudo INSTALL_DIR=$INSTALL_DIR bash scripts/install.sh
  INSTALL_DIR=\$HOME/.local/bin bash scripts/install.sh"
    fi

    info "installing $BIN_NAME to $INSTALL_DIR"
    cp "$_src_bin" "$INSTALL_DIR/$BIN_NAME"
    chmod 755 "$INSTALL_DIR/$BIN_NAME"
    info "installed: $INSTALL_DIR/$BIN_NAME"

    # Install perl-dap companion binary if present (ships since v0.9.1).
    local _src_dap="${EXTRACT_DIR}/${DAP_BIN_NAME}"
    if [ -f "$_src_dap" ]; then
        info "installing $DAP_BIN_NAME to $INSTALL_DIR"
        cp "$_src_dap" "$INSTALL_DIR/$DAP_BIN_NAME"
        chmod 755 "$INSTALL_DIR/$DAP_BIN_NAME"
        info "installed: $INSTALL_DIR/$DAP_BIN_NAME"
    fi
}

# ── Post-install checks ────────────────────────────────────────────────────────

verify_install() {
    local _bin="$INSTALL_DIR/$BIN_NAME"
    if [ ! -x "$_bin" ]; then
        err "installed binary not executable: $_bin"
    fi

    local _got_version
    if _got_version="$("$_bin" --version 2>&1)"; then
        info "verified: $_got_version"
    else
        warn "could not run '$BIN_NAME --version'; the binary may require a restart to load shared libraries"
    fi
}

check_path() {
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*)
            info "$INSTALL_DIR is in PATH"
            ;;
        *)
            warn "$INSTALL_DIR is not in PATH"
            say ""
            say "Add it by appending one of these lines to your shell's startup file:"
            say ""
            say "  bash/zsh:  export PATH=\"\$PATH:$INSTALL_DIR\""
            say "  fish:      fish_add_path $INSTALL_DIR"
            say ""
            say "Then reload your shell, or run:  export PATH=\"\$PATH:$INSTALL_DIR\""
            ;;
    esac
}

# ── Main ───────────────────────────────────────────────────────────────────────

main() {
    say ""
    say "Perl LSP installer"
    say "=================="
    say ""

    need_cmd curl
    need_cmd tar

    detect_platform
    resolve_version
    download_and_verify
    extract_archive
    install_binaries
    verify_install
    check_path

    say ""
    say "Done. ${BIN_NAME} ${VERSION_NUM} installed to ${INSTALL_DIR}/${BIN_NAME}"
    say ""
    say "Get started:"
    say "  VS Code:  install the Perl LSP extension from the marketplace"
    say "  Neovim:   add perllsp to your LSP config"
    say "  Other:    configure to use '${INSTALL_DIR}/${BIN_NAME} --stdio'"
    say ""
    say "Docs: https://github.com/$REPO"
    say ""
}

main "$@"
