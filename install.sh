#!/bin/bash
# Perl LSP installer for Linux and macOS
# Usage: curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
VERSION="${1:-latest}"
INSTALL_DIR="${2:-$HOME/.local/bin}"
REPO="EffortlessMetrics/perl-lsp"
NAME="perl-lsp"

# Functions
write_info() {
    echo -e "${BLUE}→${NC} $1"
}

write_success() {
    echo -e "${GREEN}✓${NC} $1"
}

write_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

write_error() {
    echo -e "${RED}Error:${NC} $1"
    exit 1
}

# Detect OS and architecture
detect_system() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    case $OS in
        linux)
            OS="linux"
            ;;
        darwin)
            OS="darwin"
            ;;
        *)
            write_error "Unsupported OS: $OS"
            ;;
    esac
    
    case $ARCH in
        x86_64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            write_error "Unsupported architecture: $ARCH"
            ;;
    esac
    
    TARGET="$ARCH-unknown-$OS-gnu"
    if [ "$OS" = "darwin" ]; then
        TARGET="$ARCH-apple-darwin"
    fi
    
    write_info "Detected system: $OS ($ARCH) - $TARGET"
}

# Get version information
get_version() {
    if [ "$VERSION" = "latest" ]; then
        write_info "Fetching latest release..."
        RELEASE_API="https://api.github.com/repos/$REPO/releases/latest"
        
        if ! command -v curl >/dev/null 2>&1; then
            write_error "curl is required but not installed"
        fi
        
        RELEASE_INFO=$(curl -s "$RELEASE_API")
        if [ $? -ne 0 ]; then
            write_error "Failed to fetch release information"
        fi
        
        TAG=$(echo "$RELEASE_INFO" | grep '"tag_name":' | sed -E 's/.*"tag_name": ?"([^"]+)".*/\1/')
        if [ -z "$TAG" ]; then
            write_error "Could not determine latest version"
        fi
        
        write_info "Latest version: $TAG"
    else
        TAG="v$VERSION"
        if [[ "$VERSION" == v* ]]; then
            TAG="$VERSION"
        fi
    fi
}

# Download and install binary
install_binary() {
    ASSET="$NAME-$TAG-$TARGET.tar.gz"
    URL="https://github.com/$REPO/releases/download/$TAG/$ASSET"
    
    write_info "Downloading $NAME $TAG for $TARGET"
    
    # Create temporary directory
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT
    
    # Download archive
    ARCHIVE_PATH="$TEMP_DIR/$ASSET"
    if ! curl -fsSL "$URL" -o "$ARCHIVE_PATH"; then
        write_error "Failed to download from $URL"
    fi
    
    # Download and verify checksum
    CHECKSUM_URL="https://github.com/$REPO/releases/download/$TAG/SHA256SUMS"
    CHECKSUM_PATH="$TEMP_DIR/SHA256SUMS"
    
    if curl -fsSL "$CHECKSUM_URL" -o "$CHECKSUM_PATH" 2>/dev/null; then
        EXPECTED_HASH=$(grep "$ASSET" "$CHECKSUM_PATH" | awk '{print $1}')
        ACTUAL_HASH=$(sha256sum "$ARCHIVE_PATH" | awk '{print $1}')
        
        if [ "$EXPECTED_HASH" = "$ACTUAL_HASH" ]; then
            write_success "Checksum verified"
        else
            write_warn "Checksum mismatch - expected: $EXPECTED_HASH, got: $ACTUAL_HASH"
        fi
    else
        write_warn "Could not download or verify checksums"
    fi
    
    # Extract archive
    write_info "Extracting archive"
    cd "$TEMP_DIR"
    tar xzf "$ARCHIVE_PATH"
    
    # Find the binary
    EXTRACTED_DIR="$NAME-$TAG-$TARGET"
    if [ ! -d "$EXTRACTED_DIR" ]; then
        write_error "Extracted directory not found: $EXTRACTED_DIR"
    fi
    
    BINARY_PATH="$EXTRACTED_DIR/$NAME"
    if [ ! -f "$BINARY_PATH" ]; then
        write_error "Binary not found at $BINARY_PATH"
    fi
    
    # Create install directory
    mkdir -p "$INSTALL_DIR"
    
    # Install binary
    DEST_PATH="$INSTALL_DIR/$NAME"
    write_info "Installing $NAME to $DEST_PATH"
    
    # Remove old binary if exists
    if [ -f "$DEST_PATH" ]; then
        rm -f "$DEST_PATH"
    fi
    
    # Copy binary and make executable
    cp "$BINARY_PATH" "$DEST_PATH"
    chmod +x "$DEST_PATH"
    
    write_success "Installed $NAME to $DEST_PATH"
}

# Verify installation
verify_installation() {
    if [ ! -f "$DEST_PATH" ]; then
        write_error "Installation failed - binary not found at $DEST_PATH"
    fi
    
    write_info "Verifying installation..."
    if VERSION_OUTPUT=$("$DEST_PATH" --version 2>&1); then
        write_success "Installation verified: $VERSION_OUTPUT"
    else
        write_warn "Could not verify installation"
    fi
}

# Check PATH
check_path() {
    if echo ":$PATH:" | grep -q ":$INSTALL_DIR:"; then
        write_success "$INSTALL_DIR is already in your PATH"
    else
        write_warn "$INSTALL_DIR is not in your PATH"
        echo
        echo "To add it to your PATH permanently, run:"
        echo
        if [ -n "$BASH_VERSION" ]; then
            echo "  echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> ~/.bashrc"
            echo "  source ~/.bashrc"
        elif [ -n "$ZSH_VERSION" ]; then
            echo "  echo 'export PATH=\"\$PATH:$INSTALL_DIR\"' >> ~/.zshrc"
            echo "  source ~/.zshrc"
        else
            echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
        fi
        echo
        echo "Or add it temporarily for this session:"
        echo
        echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
        echo
    fi
}

# Main installation flow
main() {
    echo
    echo "Perl Language Server Installer v1.0.0"
    echo "====================================="
    echo
    
    detect_system
    get_version
    install_binary
    verify_installation
    check_path
    
    echo
    echo "Installation complete! 🎉"
    echo
    echo "To get started with Perl LSP:"
    echo "  • VS Code: Install the Perl LSP extension from the marketplace"
    echo "  • Other editors: Configure to use '$DEST_PATH --stdio'"
    echo
    echo "For more information: https://github.com/$REPO"
    echo
}

# Run main function
main "$@"