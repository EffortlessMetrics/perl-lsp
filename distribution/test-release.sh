#!/bin/bash
# Test release artifacts locally before publishing

set -e

VERSION="${1:-1.0.0}"
PACKAGE_NAME="perl-lsp"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Testing perl-lsp release artifacts v${VERSION}${NC}"
echo ""

# Function to test binary
test_binary() {
    local binary="$1"
    local platform="$2"

    echo -e "${YELLOW}Testing $platform binary...${NC}"

    if [ ! -f "$binary" ]; then
        echo -e "${RED}Error: Binary not found: $binary${NC}"
        return 1
    fi

    # Test version
    VERSION_OUTPUT=$("$binary" --version 2>&1 || true)
    echo "  Version: $VERSION_OUTPUT"

    # Test help
    HELP_OUTPUT=$("$binary" --help 2>&1 || true)
    if [[ "$HELP_OUTPUT" == *"perl-lsp"* ]]; then
        echo -e "${GREEN}  ✓ Help output valid${NC}"
    else
        echo -e "${RED}  ✗ Help output invalid${NC}"
        return 1
    fi

    # Test that binary is executable
    if [ -x "$binary" ]; then
        echo -e "${GREEN}  ✓ Binary is executable${NC}"
    else
        echo -e "${RED}  ✗ Binary is not executable${NC}"
        return 1
    fi

    echo -e "${GREEN}  ✓ $platform binary tests passed${NC}"
    echo ""
}

# Function to test checksum
test_checksum() {
    local file="$1"
    local checksum_file="$2"

    echo -e "${YELLOW}Testing checksum for $file...${NC}"

    if [ ! -f "$checksum_file" ]; then
        echo -e "${RED}Error: Checksum file not found: $checksum_file${NC}"
        return 1
    fi

    # Verify checksum
    if command -v sha256sum >/dev/null; then
        if sha256sum -c "$checksum_file" >/dev/null 2>&1; then
            echo -e "${GREEN}  ✓ Checksum valid${NC}"
        else
            echo -e "${RED}  ✗ Checksum invalid${NC}"
            return 1
        fi
    elif command -v shasum >/dev/null; then
        if shasum -a 256 -c "$checksum_file" >/dev/null 2>&1; then
            echo -e "${GREEN}  ✓ Checksum valid${NC}"
        else
            echo -e "${RED}  ✗ Checksum invalid${NC}"
            return 1
        fi
    else
        echo -e "${YELLOW}  ⚠ No checksum tool available${NC}"
    fi
    echo ""
}

# Test if we're in a release directory
if [ -d "perl-lsp-${VERSION}-x86_64-unknown-linux-gnu" ]; then
    # Test Linux x86_64
    test_binary "perl-lsp-${VERSION}-x86_64-unknown-linux-gnu/perl-lsp" "Linux x86_64"
    test_checksum "perl-lsp-${VERSION}-x86_64-unknown-linux-gnu/perl-lsp" \
                  "perl-lsp-${VERSION}-x86_64-unknown-linux-gnu/SHA256SUMS.txt"
elif [ -d "perl-lsp-${VERSION}-x86_64-apple-darwin" ]; then
    # Test macOS x86_64
    test_binary "perl-lsp-${VERSION}-x86_64-apple-darwin/perl-lsp" "macOS x86_64"
    test_checksum "perl-lsp-${VERSION}-x86_64-apple-darwin/perl-lsp" \
                  "perl-lsp-${VERSION}-x86_64-apple-darwin/SHA256SUMS.txt"
elif [ -d "perl-lsp-${VERSION}-x86_64-pc-windows-msvc" ]; then
    # Test Windows x86_64
    test_binary "perl-lsp-${VERSION}-x86_64-pc-windows-msvc/perl-lsp.exe" "Windows x86_64"
    test_checksum "perl-lsp-${VERSION}-x86_64-pc-windows-msvc/perl-lsp.exe" \
                  "perl-lsp-${VERSION}-x86_64-pc-windows-msvc/SHA256SUMS.txt"
else
    echo -e "${YELLOW}No release directory found for v${VERSION}${NC}"
    echo ""
    echo "Usage: $0 [version]"
    echo ""
    echo "Extract release artifacts first:"
    echo "  tar xzf perl-lsp-${VERSION}-x86_64-unknown-linux-gnu.tar.gz"
    echo "  ./distribution/test-release.sh ${VERSION}"
    exit 1
fi

echo -e "${GREEN}All tests passed!${NC}"
