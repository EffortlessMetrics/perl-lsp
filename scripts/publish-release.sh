#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

VERSION="0.8.3"

echo -e "${GREEN}==================================================${NC}"
echo -e "${GREEN}     Publishing Perl Parser v${VERSION} to crates.io${NC}"
echo -e "${GREEN}==================================================${NC}"
echo

# Check if CARGO_REGISTRY_TOKEN is set
if [ -z "$CARGO_REGISTRY_TOKEN" ]; then
    echo -e "${RED}Error: CARGO_REGISTRY_TOKEN is not set${NC}"
    echo
    echo "Please set your crates.io token:"
    echo "  export CARGO_REGISTRY_TOKEN=your_token_here"
    echo
    echo "You can get your token from: https://crates.io/settings/tokens"
    exit 1
fi

# Function to publish and wait
publish_crate() {
    local crate=$1
    local wait_time=${2:-40}
    
    echo -e "${YELLOW}Publishing $crate...${NC}"
    
    if cargo publish -p "$crate" --no-verify 2>&1 | tee /tmp/publish_$crate.log; then
        echo -e "${GREEN}✓ Successfully published $crate${NC}"
        
        if [ "$wait_time" -gt 0 ]; then
            echo "Waiting ${wait_time}s for crates.io to index..."
            sleep "$wait_time"
        fi
    else
        if grep -q "already uploaded" /tmp/publish_$crate.log; then
            echo -e "${YELLOW}⚠ $crate v${VERSION} already published, skipping${NC}"
        else
            echo -e "${RED}✗ Failed to publish $crate${NC}"
            echo "Check /tmp/publish_$crate.log for details"
            exit 1
        fi
    fi
    echo
}

# Publish in dependency order
echo "Publishing crates in dependency order..."
echo

publish_crate "perl-lexer" 40
publish_crate "perl-corpus" 40
publish_crate "perl-parser" 0

echo -e "${GREEN}==================================================${NC}"
echo -e "${GREEN}           All crates published successfully!${NC}"
echo -e "${GREEN}==================================================${NC}"
echo

# Verification steps
echo "Next steps to verify the release:"
echo "1. Check crates.io:"
echo "   https://crates.io/crates/perl-lexer"
echo "   https://crates.io/crates/perl-corpus"
echo "   https://crates.io/crates/perl-parser"
echo
echo "2. Test installation:"
echo "   cargo install perl-parser --bin perl-lsp --version ${VERSION}"
echo "   perl-lsp --version"
echo
echo "3. Quick smoke test:"
cat << 'EOF' > /tmp/smoke_test.sh
#!/bin/bash
set -e
cd /tmp
rm -rf test-perl-v0.8.3
cargo new test-perl-v0.8.3 --lib
cd test-perl-v0.8.3
cargo add perl-parser@0.8.3 perl-corpus@0.8.3
cargo check
echo "✓ Smoke test passed!"
cd ..
rm -rf test-perl-v0.8.3
EOF
chmod +x /tmp/smoke_test.sh
echo "   /tmp/smoke_test.sh"
echo
echo -e "${GREEN}Release v${VERSION} complete! 🎉${NC}"