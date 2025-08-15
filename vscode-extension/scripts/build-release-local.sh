#!/bin/bash
set -euo pipefail

# Local release build script for internal distribution
# Usage: ./scripts/build-release-local.sh [version]

VERSION="${1:-v0.8.3-rc1}"
RELEASE_DIR="releases/${VERSION}"
NAME="perl-lsp"

echo "🚀 Building internal release ${VERSION}"

# Create release directory
mkdir -p "${RELEASE_DIR}"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
    linux)
        OS_NAME="linux"
        case "$ARCH" in
            x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
            aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
            *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        ;;
    darwin)
        OS_NAME="macos"
        case "$ARCH" in
            x86_64) TARGET="x86_64-apple-darwin" ;;
            arm64) TARGET="aarch64-apple-darwin" ;;
            *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo "📦 Building for ${TARGET}"

# Build the binary
echo "🔨 Building perl-lsp..."
cargo build --release -p perl-parser --bin perl-lsp

# Create package
PKG_NAME="${NAME}-${VERSION}-${TARGET}"
PKG_DIR="${RELEASE_DIR}/${PKG_NAME}"

mkdir -p "${PKG_DIR}"
cp "target/release/${NAME}" "${PKG_DIR}/"
cp README.md LICENSE "${PKG_DIR}/" 2>/dev/null || true

# Strip binary for size
strip "${PKG_DIR}/${NAME}" 2>/dev/null || true

# Create archive
cd "${RELEASE_DIR}"
tar czf "${PKG_NAME}.tar.gz" "${PKG_NAME}"
rm -rf "${PKG_NAME}"

# Generate checksum
sha256sum "${PKG_NAME}.tar.gz" > "${PKG_NAME}.tar.gz.sha256"

# Display results
echo ""
echo "✅ Release package created:"
echo "   ${RELEASE_DIR}/${PKG_NAME}.tar.gz"
echo ""
echo "📊 Package info:"
ls -lh "${PKG_NAME}.tar.gz"
cat "${PKG_NAME}.tar.gz.sha256"

cd - > /dev/null

echo ""
echo "🎯 Next steps:"
echo "1. Test: tar -xzf ${RELEASE_DIR}/${PKG_NAME}.tar.gz -C /tmp && /tmp/${PKG_NAME}/perl-lsp --version"
echo "2. Install: sudo cp target/release/perl-lsp /usr/local/bin/"
echo "3. Share: Copy ${RELEASE_DIR}/${PKG_NAME}.tar.gz to internal distribution"