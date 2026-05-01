#!/usr/bin/env bash
set -euo pipefail

REPO="EffortlessMetrics/perl-lsp"
BIN_NAME="perllsp"
DAP_BIN_NAME="perl-dap"
VERSION="${VERSION:-latest}"
PREFER_GNU="${PREFER_GNU:-0}" # legacy compatibility override
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

RED='\033[0;31m'; GREEN='\033[0;32m'; NC='\033[0m'
say(){ printf '%b\n' "$1"; }
info(){ say "${GREEN}=>${NC} $1"; }
err(){ say "${RED}error:${NC} $1" >&2; exit 1; }

normalize_linux_libc_choice() {
  case "${1:-auto}" in
    auto|"") echo auto ;;
    glibc|gnu) echo gnu ;;
    musl) echo musl ;;
    *) err "invalid PERL_LSP_LINUX_LIBC value: $1
Expected one of: auto, glibc, gnu, musl" ;;
  esac
}

detect_linux_libc() {
  local choice
  normalize_linux_libc_choice "$PERL_LSP_LINUX_LIBC" >/dev/null
  choice="$(normalize_linux_libc_choice "$PERL_LSP_LINUX_LIBC")"
  if [ "$choice" != "auto" ]; then echo "$choice"; return; fi
  if [ "$PREFER_GNU" = "1" ]; then echo gnu; return; fi
  if [ -f /etc/alpine-release ]; then echo musl; return; fi
  if command -v ldd >/dev/null 2>&1 && ldd --version 2>&1 | grep -qi musl; then echo musl; return; fi
  if ls /lib/ld-musl-*.so.1 /usr/lib/ld-musl-*.so.1 >/dev/null 2>&1; then echo musl; return; fi
  echo gnu
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

detect_platform(){
  local os arch termux=0 libc
  os="$(uname -s)"; arch="$(uname -m)"
  [ -n "${TERMUX_VERSION:-}" ] || [ -d "/data/data/com.termux/files/usr/bin" ] && termux=1
  [ "$termux" = "1" ] && err "Termux/Android does not currently have a pre-built release asset.

Install from source instead:
  pkg install rust
  cargo install perllsp

Then configure your editor to run:
  perllsp --stdio"
  case "$arch" in x86_64|amd64|x64) arch=x86_64;; aarch64|arm64) arch=aarch64;; *) err "unsupported architecture: $arch";; esac
  case "$os" in
    Linux) libc="$(detect_linux_libc)"; TARGET="${arch}-unknown-linux-${libc}" ;;
    Darwin) TARGET="${arch}-apple-darwin" ;;
    *) err "unsupported operating system: $os" ;;
  esac
  info "detected: $(describe_target "$TARGET")"
  info "selected release asset target: $TARGET"
}

resolve_version(){
  if [ "$VERSION" = latest ]; then
    TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)"
    [ -n "$TAG" ] || err "could not resolve latest release"
  else
    case "$VERSION" in v*) TAG="$VERSION";; *) TAG="v$VERSION";; esac
  fi
  VERSION_NUM="${TAG#v}"; info "version: $TAG"
}

main(){
  say "Perl LSP installer"; say "=================="; say ""
  detect_platform
  if [ "$PRINT_TARGET" = "1" ]; then printf '%s\n' "$TARGET"; exit 0; fi
  resolve_version
  asset="${BIN_NAME}-${VERSION_NUM}-${TARGET}.tar.gz"
  url="https://github.com/${REPO}/releases/download/${TAG}/${asset}"
  tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
  info "downloading ${asset}"
  curl -fsSL "$url" -o "$tmp/$asset" || err "download failed: $url"
  tar -xzf "$tmp/$asset" -C "$tmp"
  dir="$tmp/${BIN_NAME}-${VERSION_NUM}-${TARGET}"
  [ -f "$dir/$BIN_NAME" ] || err "binary missing in archive"
  mkdir -p "$INSTALL_DIR"
  install -m 755 "$dir/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME"
  if [ -f "$dir/$DAP_BIN_NAME" ]; then install -m 755 "$dir/$DAP_BIN_NAME" "$INSTALL_DIR/$DAP_BIN_NAME"; fi
  info "installed $BIN_NAME to $INSTALL_DIR/$BIN_NAME"
}

main "$@"
