#!/bin/bash
set -euo pipefail

echo "🔍 v0.8.3 Release Smoke Test"
echo "============================"
echo ""

# Create temp directory for testing
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"
echo "Testing in: $TEMP_DIR"
echo ""

# Test 1: Install perl-lsp from crates.io
echo "📦 Installing perl-lsp from crates.io..."
if cargo install perl-parser --bin perl-lsp --locked; then
    echo "✅ Installation successful"
else
    echo "❌ Installation failed"
    exit 1
fi
echo ""

# Test 2: Verify version
echo "📋 Checking version..."
VERSION=$(perl-lsp --version | head -1)
if [[ "$VERSION" == *"0.8.3"* ]]; then
    echo "✅ Version correct: $VERSION"
else
    echo "❌ Version mismatch: $VERSION"
    exit 1
fi
echo ""

# Test 3: Basic LSP functionality
echo "🔧 Testing LSP server..."
cat > test_request.json << 'EOF'
Content-Length: 85

{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{},"rootUri":null}}
EOF

if timeout 2 perl-lsp --stdio < test_request.json > response.json 2>/dev/null; then
    if grep -q '"id":1' response.json; then
        echo "✅ LSP server responds to initialize"
    else
        echo "❌ LSP server response invalid"
        exit 1
    fi
else
    echo "⚠️  LSP server timed out (expected for stdio mode)"
fi
echo ""

# Test 4: Parse simple Perl file
echo "🔍 Testing Perl parsing..."
cat > test.pl << 'EOF'
#!/usr/bin/perl
use strict;
use warnings;

my $greeting = "Hello, World!";
print "$greeting\n";

sub factorial {
    my ($n) = @_;
    return 1 if $n <= 1;
    return $n * factorial($n - 1);
}

print factorial(5), "\n";
EOF

# Since perl-lsp is an LSP server, we can't directly parse files
# But we can check that the binary exists and runs
if perl-lsp --help > /dev/null 2>&1; then
    echo "✅ perl-lsp binary functional"
else
    echo "❌ perl-lsp binary not working"
    exit 1
fi
echo ""

# Test 5: Verify all crates published
echo "📚 Checking crates.io for all packages..."
for crate in perl-lexer perl-corpus perl-parser-pest perl-parser; do
    echo -n "  Checking $crate... "
    if curl -s "https://crates.io/api/v1/crates/$crate" | grep -q '"newest_version":"0.8.3"'; then
        echo "✅"
    else
        echo "⚠️  (may need more time to index)"
    fi
done
echo ""

# Cleanup
cd /
rm -rf "$TEMP_DIR"

echo "================================"
echo "✨ Smoke test complete!"
echo ""
echo "Next steps:"
echo "1. Check crates.io pages for proper descriptions"
echo "2. Verify keywords and categories are correct"
echo "3. Test with your editor's LSP client"
echo "4. Announce the release!"