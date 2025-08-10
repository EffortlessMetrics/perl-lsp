#!/usr/bin/env bash
set -euo pipefail

# Perl Language Server Installer
# One-liner: curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash

REPO="${REPO:-EffortlessSteven/tree-sitter-perl}"
VERSION="${VERSION:-latest}"   # e.g. 0.7.5 or "latest"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

error() { echo -e "${RED}Error: $1${NC}" >&2; exit 1; }
info() { echo -e "${GREEN}➜${NC} $1"; }
warn() { echo -e "${YELLOW}⚠${NC} $1"; }

detect_platform() {
  local os arch
  case "$(uname -s)" in
    Linux)  os=linux ;;
    Darwin) os=darwin ;;
    MINGW*|MSYS*|CYGWIN*) error "Windows not supported by this shell script. Download the .zip file directly." ;;
    *) error "Unsupported OS: $(uname -s)" ;;
  esac
  
  case "$(uname -m)" in
    x86_64|amd64) arch=x86_64 ;;
    arm64|aarch64) arch=aarch64 ;;
    *) error "Unsupported architecture: $(uname -m)" ;;
  esac
  
  echo "${os}-${arch}"
}

# Map our platform to cargo-dist target triple
platform_to_target() {
  case "$1" in
    linux-x86_64)  echo "x86_64-unknown-linux-gnu" ;;
    linux-aarch64) echo "aarch64-unknown-linux-gnu" ;;
    darwin-x86_64) echo "x86_64-apple-darwin" ;;
    darwin-aarch64) echo "aarch64-apple-darwin" ;;
    *) error "Unknown platform: $1" ;;
  esac
}

PLATFORM="$(detect_platform)"
TARGET="$(platform_to_target "$PLATFORM")"

info "Detected platform: $PLATFORM (target: $TARGET)"

# Determine download URL based on version
if [[ "$VERSION" == "latest" ]]; then
  # Try different asset naming patterns
  BASE_URL="https://github.com/${REPO}/releases/latest/download"
  
  # Common patterns: perl-lsp-{target}.tar.gz, perl-lsp-v{version}-{target}.tar.gz, etc.
  for pattern in \
    "perl-lsp-${TARGET}.tar.gz" \
    "perl-lsp-${TARGET}.tar.xz" \
    "perl-lsp-${PLATFORM}.tar.gz" \
    "perl-lsp-v${VERSION}-${TARGET}.tar.gz" \
    "perl-lsp-v${VERSION}-${TARGET}.tar.xz"
  do
    URL="${BASE_URL}/${pattern}"
    if curl -fsLI "$URL" >/dev/null 2>&1; then
      DOWNLOAD_URL="$URL"
      break
    fi
  done
  
  if [[ -z "${DOWNLOAD_URL:-}" ]]; then
    warn "Could not find release asset. Trying API..."
    # Fallback to API
    DOWNLOAD_URL=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | \
      grep -E "browser_download_url.*${TARGET}|browser_download_url.*${PLATFORM}" | \
      head -1 | cut -d '"' -f 4)
  fi
else
  # Specific version - try common patterns
  BASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"
  
  for pattern in \
    "perl-lsp-${TARGET}.tar.gz" \
    "perl-lsp-${TARGET}.tar.xz" \
    "perl-lsp-v${VERSION}-${TARGET}.tar.gz" \
    "perl-lsp-v${VERSION}-${TARGET}.tar.xz" \
    "perl-lsp-${PLATFORM}.tar.gz"
  do
    URL="${BASE_URL}/${pattern}"
    if curl -fsLI "$URL" >/dev/null 2>&1; then
      DOWNLOAD_URL="$URL"
      break
    fi
  done
fi

if [[ -z "${DOWNLOAD_URL:-}" ]]; then
  error "Could not find download URL for platform: $PLATFORM (target: $TARGET)
  
Please check:
  1. The release exists at https://github.com/${REPO}/releases
  2. There's an asset for $TARGET
  3. Or download manually and place in $INSTALL_DIR"
fi

# Determine archive format
case "$DOWNLOAD_URL" in
  *.tar.gz|*.tgz) EXTRACT_CMD="tar -xzf" ;;
  *.tar.xz) EXTRACT_CMD="tar -xJf" ;;
  *.tar.bz2) EXTRACT_CMD="tar -xjf" ;;
  *.zip) EXTRACT_CMD="unzip -q" ;;
  *) error "Unknown archive format: $DOWNLOAD_URL" ;;
esac

mkdir -p "$INSTALL_DIR"

# Use temp directory for safe extraction
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

info "Downloading perl-lsp from: ${DOWNLOAD_URL##*/}"
curl -fL --retry 3 --progress-bar -o "$tmp/archive" "$DOWNLOAD_URL" || \
  error "Failed to download from $DOWNLOAD_URL"

# Optional checksum verification
CHECKSUM_URL="${DOWNLOAD_URL}.sha256"
if curl -fsL -o "$tmp/archive.sha256" "$CHECKSUM_URL" 2>/dev/null; then
  info "Verifying checksum..."
  
  # Extract just the hash from the checksum file (handles different formats)
  expected_hash=$(awk '{print $1}' "$tmp/archive.sha256")
  
  if command -v sha256sum >/dev/null 2>&1; then
    actual_hash=$(sha256sum "$tmp/archive" | awk '{print $1}')
  elif command -v shasum >/dev/null 2>&1; then
    actual_hash=$(shasum -a 256 "$tmp/archive" | awk '{print $1}')
  else
    warn "No sha256 tool found; skipping verification"
    actual_hash=""
  fi
  
  if [[ -n "$actual_hash" ]]; then
    if [[ "$expected_hash" == "$actual_hash" ]]; then
      info "✓ Checksum verified"
    else
      error "Checksum mismatch!
  Expected: $expected_hash
  Got:      $actual_hash"
    fi
  fi
else
  warn "No checksum file available (this is OK)"
fi

info "Extracting archive..."
cd "$tmp"
$EXTRACT_CMD archive || error "Failed to extract archive"

# Install binaries (handles both flat and nested archive structures)
install_bin() {
  local name="$1"
  local installed=false
  
  # Look for the binary in common locations
  for location in \
    "$name" \
    "*/bin/$name" \
    "*/$name" \
    "perl-lsp-${TARGET}/$name" \
    "perl-lsp-v${VERSION}-${TARGET}/$name" \
    "release/$name"
  do
    for path in $location; do
      if [[ -f "$path" && -x "$path" ]]; then
        install -m 755 "$path" "$INSTALL_DIR/$name"
        info "Installed $name → $INSTALL_DIR/$name"
        installed=true
        break 2
      fi
    done
  done
  
  if [[ "$installed" == false && "$name" == "perl-lsp" ]]; then
    error "Could not find $name binary in archive"
  fi
}

install_bin perl-lsp
install_bin perl-parse  # optional companion binary if shipped

# Verify installation
if [[ -x "$INSTALL_DIR/perl-lsp" ]]; then
  VERSION_OUTPUT=$("$INSTALL_DIR/perl-lsp" --version 2>&1 || echo "unknown")
  info "perl-lsp installed successfully! Version: $VERSION_OUTPUT"
else
  error "Installation failed - binary not executable"
fi

# PATH configuration detection
case ":$PATH:" in
  *":$INSTALL_DIR:"*)
    info "✓ $INSTALL_DIR is already in your PATH"
    ;;
  *)
    warn "$INSTALL_DIR is not on your PATH"
    
    # Detect shell config file
    shell_rc=""
    shell_name="${SHELL##*/}"
    
    case "$shell_name" in
      zsh)
        if [[ -n "${ZDOTDIR:-}" && -f "$ZDOTDIR/.zshrc" ]]; then
          shell_rc="$ZDOTDIR/.zshrc"
        elif [[ -f "$HOME/.zshrc" ]]; then
          shell_rc="$HOME/.zshrc"
        fi
        ;;
      bash)
        if [[ -f "$HOME/.bashrc" ]]; then
          shell_rc="$HOME/.bashrc"
        elif [[ -f "$HOME/.bash_profile" ]]; then
          shell_rc="$HOME/.bash_profile"
        fi
        ;;
      fish)
        shell_rc="$HOME/.config/fish/config.fish"
        ;;
      *)
        if [[ -f "$HOME/.profile" ]]; then
          shell_rc="$HOME/.profile"
        fi
        ;;
    esac
    
    echo
    if [[ -n "$shell_rc" ]]; then
      echo "Add this line to $shell_rc:"
    else
      echo "Add this line to your shell config:"
    fi
    
    if [[ "$shell_name" == "fish" ]]; then
      echo "  set -x PATH $INSTALL_DIR \$PATH"
    else
      echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    fi
    echo
    echo "Then reload your shell or run:"
    echo "  source ${shell_rc:-~/.profile}"
    ;;
esac

echo
echo "🎉 Installation complete!"
echo
echo "Configure your editor:"
echo "  VSCode:  Install 'Perl Language Server' extension"
echo "  Neovim:  Add to lspconfig with cmd = {'perl-lsp', '--stdio'}"
echo "  Emacs:   Configure eglot or lsp-mode"
echo "  Sublime: Install via Package Control"
echo
echo "Test the installation:"
echo "  perl-lsp --version"
echo
echo "Documentation: https://github.com/${REPO}#readme"