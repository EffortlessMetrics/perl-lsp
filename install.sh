#!/bin/bash
set -e

# Perl LSP Quick Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/yourusername/tree-sitter-perl/main/install.sh | bash

REPO="yourusername/tree-sitter-perl"
VERSION=${1:-latest}

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  darwin)
    OS_NAME="apple-darwin"
    ;;
  linux)
    OS_NAME="unknown-linux-gnu"
    ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64)
    ARCH_NAME="x86_64"
    ;;
  arm64|aarch64)
    ARCH_NAME="aarch64"
    ;;
  *)
    echo "Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

PLATFORM="${ARCH_NAME}-${OS_NAME}"

# Get the latest release URL
if [ "$VERSION" = "latest" ]; then
  DOWNLOAD_URL=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep "browser_download_url.*${PLATFORM}" \
    | cut -d '"' -f 4)
else
  DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/perl-lsp-${PLATFORM}.tar.gz"
fi

if [ -z "$DOWNLOAD_URL" ]; then
  echo "Could not find download URL for platform: $PLATFORM"
  exit 1
fi

# Create install directory
INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"

# Download and extract
echo "Downloading perl-lsp for ${PLATFORM}..."
curl -fsSL "$DOWNLOAD_URL" | tar xz -C "$INSTALL_DIR"

# Make executable
chmod +x "${INSTALL_DIR}/perl-lsp"
[ -f "${INSTALL_DIR}/perl-parse" ] && chmod +x "${INSTALL_DIR}/perl-parse"

echo "✅ perl-lsp installed to ${INSTALL_DIR}"
echo ""
echo "Add this to your shell profile:"
echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
echo ""
echo "Configure your editor:"
echo "  VSCode: Install 'Perl Language Server' extension"
echo "  Neovim: Add to lspconfig with cmd = {'perl-lsp', '--stdio'}"
echo "  Emacs: Configure eglot or lsp-mode"