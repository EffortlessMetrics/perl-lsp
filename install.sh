#!/bin/bash
set -e

# Perl Language Server Installation Script
# Supports Linux, macOS, and Windows (via WSL)

VERSION="0.6.0"
REPO="anthropics/perl-language-server"
BASE_URL="https://github.com/$REPO/releases/download/v$VERSION"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored output
print_info() { echo -e "${BLUE}ℹ${NC} $1"; }
print_success() { echo -e "${GREEN}✓${NC} $1"; }
print_error() { echo -e "${RED}✗${NC} $1"; }
print_warning() { echo -e "${YELLOW}⚠${NC} $1"; }

# Detect OS and architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    case "$OS" in
        linux*)
            PLATFORM="linux"
            ;;
        darwin*)
            PLATFORM="macos"
            ;;
        mingw*|msys*|cygwin*)
            PLATFORM="windows"
            ;;
        *)
            print_error "Unsupported operating system: $OS"
            exit 1
            ;;
    esac
    
    case "$ARCH" in
        x86_64|amd64)
            ARCH="x64"
            ;;
        aarch64|arm64)
            ARCH="arm64"
            ;;
        *)
            print_error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac
    
    print_info "Detected platform: $PLATFORM-$ARCH"
}

# Check for required tools
check_requirements() {
    local missing=()
    
    if ! command -v curl &> /dev/null && ! command -v wget &> /dev/null; then
        missing+=("curl or wget")
    fi
    
    if ! command -v tar &> /dev/null; then
        missing+=("tar")
    fi
    
    if [ ${#missing[@]} -ne 0 ]; then
        print_error "Missing required tools: ${missing[*]}"
        print_info "Please install them and try again."
        exit 1
    fi
}

# Download file
download() {
    local url=$1
    local output=$2
    
    if command -v curl &> /dev/null; then
        curl -fsSL "$url" -o "$output"
    elif command -v wget &> /dev/null; then
        wget -q "$url" -O "$output"
    else
        print_error "No download tool available"
        return 1
    fi
}

# Install binaries
install_binaries() {
    local install_dir="${INSTALL_DIR:-$HOME/.local/bin}"
    local temp_dir=$(mktemp -d)
    
    print_info "Installing to: $install_dir"
    
    # Create install directory if it doesn't exist
    mkdir -p "$install_dir"
    
    # Download archive
    local archive_name="perl-lsp-$VERSION-$PLATFORM-$ARCH.tar.gz"
    local download_url="$BASE_URL/$archive_name"
    
    print_info "Downloading $archive_name..."
    if ! download "$download_url" "$temp_dir/$archive_name"; then
        print_error "Failed to download from $download_url"
        print_info "Please check if the release exists or try building from source."
        rm -rf "$temp_dir"
        exit 1
    fi
    
    # Extract archive
    print_info "Extracting binaries..."
    tar -xzf "$temp_dir/$archive_name" -C "$temp_dir"
    
    # Install binaries
    for binary in perl-lsp perl-dap; do
        if [ -f "$temp_dir/$binary" ]; then
            print_info "Installing $binary..."
            install -m 755 "$temp_dir/$binary" "$install_dir/$binary"
            print_success "$binary installed"
        fi
    done
    
    # Clean up
    rm -rf "$temp_dir"
    
    # Check if install directory is in PATH
    if [[ ":$PATH:" != *":$install_dir:"* ]]; then
        print_warning "$install_dir is not in your PATH"
        print_info "Add the following to your shell configuration:"
        echo "    export PATH=\"$install_dir:\$PATH\""
    fi
}

# Build from source
build_from_source() {
    print_info "Building from source..."
    
    # Check for Rust
    if ! command -v cargo &> /dev/null; then
        print_error "Rust is not installed"
        print_info "Install Rust from https://rustup.rs/"
        exit 1
    fi
    
    # Clone repository if not in it
    if [ ! -f "Cargo.toml" ]; then
        print_info "Cloning repository..."
        git clone "https://github.com/$REPO.git" perl-language-server
        cd perl-language-server
    fi
    
    # Build binaries
    print_info "Building perl-lsp..."
    cargo build --release -p perl-parser --bin perl-lsp
    
    print_info "Building perl-dap..."
    cargo build --release -p perl-parser --bin perl-dap
    
    # Install binaries
    local install_dir="${INSTALL_DIR:-$HOME/.local/bin}"
    mkdir -p "$install_dir"
    
    install -m 755 target/release/perl-lsp "$install_dir/perl-lsp"
    install -m 755 target/release/perl-dap "$install_dir/perl-dap"
    
    print_success "Built and installed from source"
}

# Install VSCode extension
install_vscode_extension() {
    if ! command -v code &> /dev/null; then
        print_warning "VSCode CLI not found, skipping extension installation"
        print_info "You can install the extension manually from the marketplace"
        return
    fi
    
    print_info "Installing VSCode extension..."
    
    local vsix_url="$BASE_URL/perl-language-server-$VERSION.vsix"
    local temp_vsix=$(mktemp --suffix=.vsix)
    
    if download "$vsix_url" "$temp_vsix"; then
        code --install-extension "$temp_vsix"
        rm "$temp_vsix"
        print_success "VSCode extension installed"
    else
        print_warning "Failed to download VSCode extension"
        print_info "Install manually from: https://marketplace.visualstudio.com/items?itemName=perl.language-server"
    fi
}

# Main installation flow
main() {
    echo "🚀 Perl Language Server Installer v$VERSION"
    echo "========================================="
    echo
    
    # Parse arguments
    local build_from_source=false
    local skip_vscode=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --build|--source)
                build_from_source=true
                shift
                ;;
            --no-vscode)
                skip_vscode=true
                shift
                ;;
            --help|-h)
                echo "Usage: $0 [options]"
                echo "Options:"
                echo "  --build, --source    Build from source instead of downloading"
                echo "  --no-vscode         Skip VSCode extension installation"
                echo "  --help, -h          Show this help message"
                echo
                echo "Environment variables:"
                echo "  INSTALL_DIR         Installation directory (default: ~/.local/bin)"
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Check requirements
    check_requirements
    
    # Detect platform
    detect_platform
    
    # Install binaries
    if [ "$build_from_source" = true ]; then
        build_from_source
    else
        install_binaries
    fi
    
    # Install VSCode extension
    if [ "$skip_vscode" = false ]; then
        install_vscode_extension
    fi
    
    # Verify installation
    echo
    print_info "Verifying installation..."
    
    if command -v perl-lsp &> /dev/null; then
        local version=$(perl-lsp --version 2>&1 | head -n1)
        print_success "perl-lsp: $version"
    else
        print_error "perl-lsp not found in PATH"
    fi
    
    if command -v perl-dap &> /dev/null; then
        print_success "perl-dap: installed"
    else
        print_error "perl-dap not found in PATH"
    fi
    
    echo
    print_success "Installation complete!"
    print_info "Get started: https://github.com/$REPO#quick-start"
}

# Run main function
main "$@"