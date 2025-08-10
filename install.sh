#!/usr/bin/env bash
set -euo pipefail

# Perl Language Server Installer
# One-liner: curl -fsSL https://raw.githubusercontent.com/tree-sitter-perl/perl-language-server/main/install.sh | bash

REPO="${REPO:-tree-sitter-perl/perl-language-server}"
VERSION="${VERSION:-latest}"   # e.g. 0.7.5 or "latest"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

detect_platform() {
  local os arch
  case "$(uname -s)" in
    Linux)  os=linux ;;
    Darwin) os=macos ;;
    MINGW*|MSYS*|CYGWIN*) echo "Windows not supported by this shell script"; exit 1 ;;
    *) echo "Unsupported OS: $(uname -s)"; exit 1 ;;
  esac
  case "$(uname -m)" in
    x86_64|amd64) arch=x86_64 ;;
    arm64|aarch64) arch=aarch64 ;;
    *) echo "Unsupported architecture: $(uname -m)"; exit 1 ;;
  esac
  echo "${os}-${arch}"
}

PLATFORM="$(detect_platform)"

# Build download URLs (uses redirect for latest, avoiding API rate limits)
if [[ "$VERSION" == "latest" ]]; then
  DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/perl-lsp-${PLATFORM}.tar.gz"
  CHECKSUM_URL="https://github.com/${REPO}/releases/latest/download/perl-lsp-${PLATFORM}.tar.gz.sha256"
else
  DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/perl-lsp-${PLATFORM}.tar.gz"
  CHECKSUM_URL="https://github.com/${REPO}/releases/download/v${VERSION}/perl-lsp-${PLATFORM}.tar.gz.sha256"
fi

mkdir -p "$INSTALL_DIR"

# Use temp directory for safe extraction
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "⬇️  Downloading perl-lsp (${PLATFORM}, ${VERSION})..."
curl -fL --retry 3 -o "$tmp/pkg.tgz" "$DOWNLOAD_URL"

# Optional checksum verification (if checksums are published)
if curl -fsL -o "$tmp/pkg.tgz.sha256" "$CHECKSUM_URL" 2>/dev/null; then
  echo "🔐 Verifying checksum..."
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$tmp" && sha256sum -c pkg.tgz.sha256)
  elif command -v shasum >/dev/null 2>&1; then
    (cd "$tmp" && shasum -a 256 -c pkg.tgz.sha256)
  else
    echo "⚠️  No sha256 tool found; skipping verification"
  fi
fi

echo "📦 Extracting..."
tar -xzf "$tmp/pkg.tgz" -C "$tmp"

# Install binaries (handles both flat and nested archive structures)
install_bin() {
  local name="$1"
  local path
  if [[ -f "$tmp/$name" ]]; then
    path="$tmp/$name"
  else
    path="$(find "$tmp" -maxdepth 2 -type f -name "$name" | head -n1 || true)"
  fi
  if [[ -n "${path:-}" && -f "$path" ]]; then
    install -m 755 "$path" "$INSTALL_DIR/$name"
    echo "✅ Installed $name → $INSTALL_DIR/$name"
  fi
}

install_bin perl-lsp
install_bin perl-parse  # optional companion binary if shipped

# PATH configuration detection
case ":$PATH:" in
  *":$INSTALL_DIR:"*)
    echo "✅ $INSTALL_DIR is already in your PATH"
    ;;
  *)
    echo
    echo "ℹ️  $INSTALL_DIR is not on your PATH."
    
    # Detect shell config file
    shell_rc=""
    if [[ -n "${ZDOTDIR:-}" && -f "$ZDOTDIR/.zshrc" ]]; then
      shell_rc="$ZDOTDIR/.zshrc"
    elif [[ -f "$HOME/.zshrc" ]]; then
      shell_rc="$HOME/.zshrc"
    elif [[ -f "$HOME/.bashrc" ]]; then
      shell_rc="$HOME/.bashrc"
    elif [[ -f "$HOME/.profile" ]]; then
      shell_rc="$HOME/.profile"
    fi
    
    if [[ -n "$shell_rc" ]]; then
      echo "Add this line to $shell_rc:"
    else
      echo "Add this line to your shell config:"
    fi
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac

echo
echo "🎉 Done! Try: perl-lsp --version"
echo
echo "Configure your editor:"
echo "  VSCode: Install 'Perl Language Server' extension"
echo "  Neovim: Add to lspconfig with cmd = {'perl-lsp', '--stdio'}"
echo "  Emacs:  Configure eglot or lsp-mode"