#!/usr/bin/env bash
set -euo pipefail

# Perl LSP installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/main/install.sh | bash

REPO="${REPO:-EffortlessMetrics/perl-lsp}"
NAME="perl-lsp"
VERSION="${VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

error() {
    echo -e "${RED}Error: $1${NC}" >&2
    exit 1
}

info() {
    echo -e "${GREEN}→${NC} $1"
}

warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

success() {
    echo -e "${GREEN}✓${NC} $1"
}

detect_os() {
    case "$(uname -s)" in
        Linux*)
            OS="linux"
            # Check if musl or glibc
            if ldd --version 2>&1 | grep -q musl; then
                LIBC="musl"
            else
                LIBC="gnu"
            fi
            ;;
        Darwin*)
            OS="darwin"
            LIBC=""
            ;;
        MINGW*|MSYS*|CYGWIN*)
            error "Windows not supported by this script. Please use install.ps1 instead."
            ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        arm64|aarch64)
            ARCH="aarch64"
            ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            ;;
    esac
}

get_target() {
    case "$OS" in
        linux)
            if [ "$LIBC" = "musl" ]; then
                TARGET="${ARCH}-unknown-linux-musl"
            else
                TARGET="${ARCH}-unknown-linux-gnu"
            fi
            ;;
        darwin)
            TARGET="${ARCH}-apple-darwin"
            ;;
    esac
}

detect_os
detect_arch
get_target

info "Detected system: $OS ($ARCH) - $TARGET"

get_latest_version() {
    if [ "$VERSION" = "latest" ]; then
        TAG=$(curl -sSfL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')
        if [ -z "$TAG" ]; then
            error "Failed to fetch latest release version"
        fi
    else
        TAG="$VERSION"
    fi
}

get_latest_version

# Download binary
EXT="tar.gz"
ASSET="${NAME}-${TAG}-${TARGET}.${EXT}"
URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"

info "Downloading $NAME $TAG for $TARGET"

# Create temp directory
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT
cd "$TMP_DIR"

# Download binary archive
if ! curl -sSfL "$URL" -o "$ASSET"; then
    error "Failed to download binary from $URL"
fi

# Download and verify checksums
CHECKSUM_URL="https://github.com/${REPO}/releases/download/${TAG}/SHA256SUMS"
if curl -sSfL "$CHECKSUM_URL" -o SHA256SUMS 2>/dev/null; then
    # Verify checksum
    if command -v sha256sum >/dev/null 2>&1; then
        if grep "$ASSET" SHA256SUMS | sha256sum -c - >/dev/null 2>&1; then
            success "Checksum verified"
        else
            warn "Checksum verification failed"
        fi
    elif command -v shasum >/dev/null 2>&1; then
        if grep "$ASSET" SHA256SUMS | shasum -a 256 -c - >/dev/null 2>&1; then
            success "Checksum verified"
        else
            warn "Checksum verification failed"
        fi
    fi
else
    warn "Could not download checksums for verification"
fi

# Extract archive
info "Extracting archive"
tar xzf "$ASSET"

# Find the extracted directory
EXTRACT_DIR="${NAME}-${TAG}-${TARGET}"
if [ ! -d "$EXTRACT_DIR" ]; then
    error "Expected directory $EXTRACT_DIR not found after extraction"
fi

cd "$EXTRACT_DIR"

# Install binary
BIN="${NAME}"

# Check if binary exists
if [ ! -f "$BIN" ]; then
    error "Binary $BIN not found in extracted archive"
fi

# Create install directory if it doesn't exist
mkdir -p "$INSTALL_DIR"

# Install binary
info "Installing $NAME to $INSTALL_DIR"

# Remove old binary if it exists
if [ -f "$INSTALL_DIR/$NAME" ]; then
    rm "$INSTALL_DIR/$NAME"
fi

# Copy and make executable
cp "$BIN" "$INSTALL_DIR/$NAME"
chmod +x "$INSTALL_DIR/$NAME"

success "Installed $NAME to $INSTALL_DIR/$NAME"

# Verify installation
if "$INSTALL_DIR/$NAME" --version >/dev/null 2>&1; then
    VERSION_OUTPUT=$("$INSTALL_DIR/$NAME" --version 2>&1 || true)
    success "Installation verified: $VERSION_OUTPUT"
else
    warn "Could not verify installation. You may need to restart your terminal."
fi

# Check if install directory is in PATH
case ":$PATH:" in
    *":$INSTALL_DIR:"*)
        success "$INSTALL_DIR is already in your PATH"
        ;;
    *)
        warn "$INSTALL_DIR is not in your PATH"
        echo ""
        echo "Add it to your PATH by adding this line to your shell config:"
        echo ""
        
        # Detect shell and provide appropriate instruction
        if [ -n "${BASH_VERSION:-}" ]; then
            echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc"
        elif [ -n "${ZSH_VERSION:-}" ]; then
            echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.zshrc"
        elif [ -n "${FISH_VERSION:-}" ]; then
            echo "  echo 'set -gx PATH $INSTALL_DIR \$PATH' >> ~/.config/fish/config.fish"
        else
            echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        fi
        echo ""
        echo "Then reload your shell configuration or start a new terminal."
        ;;
esac

echo ""
echo "Installation complete! 🎉"
echo ""
echo "To get started with Perl LSP:"
echo "  • VS Code: Install the Perl LSP extension from the marketplace"
echo "  • Neovim: Add perl-lsp to your LSP config"
echo "  • Other editors: Configure to use '$INSTALL_DIR/$NAME --stdio'"
echo ""
echo "For more information: https://github.com/$REPO"