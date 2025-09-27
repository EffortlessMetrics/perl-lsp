#!/bin/bash
set -euo pipefail

echo "🚀 Publishing v0.8.3 GA to crates.io"
echo "===================================="
echo ""

# Check if CARGO_REGISTRY_TOKEN is set
if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
    echo "❌ Error: CARGO_REGISTRY_TOKEN is not set"
    echo ""
    echo "Please run:"
    echo "  export CARGO_REGISTRY_TOKEN=your_crates_io_token"
    echo ""
    exit 1
fi

echo "✅ Token found"
echo ""

# Function to publish and wait
publish_crate() {
    local crate=$1
    local wait_time=${2:-40}
    
    echo "📦 Publishing $crate..."
    cd "crates/$crate"
    
    if cargo publish; then
        echo "✅ $crate published successfully"
        if [ "$wait_time" -gt 0 ]; then
            echo "⏳ Waiting ${wait_time}s for crates.io indexing..."
            sleep "$wait_time"
        fi
    else
        echo "❌ Failed to publish $crate"
        exit 1
    fi
    
    cd ../..
    echo ""
}

# Publish in dependency order
publish_crate "perl-lexer" 40
publish_crate "perl-corpus" 40
publish_crate "perl-parser-pest" 40
publish_crate "perl-parser" 0  # No wait needed for last crate

echo ""
echo "✨ All crates published successfully!"
echo ""
echo "🔍 Running smoke test in 60 seconds..."
sleep 60

# Run smoke test
if [ -x "./scripts/smoke-test-release.sh" ]; then
    ./scripts/smoke-test-release.sh
else
    echo "⚠️  Smoke test script not found or not executable"
fi

echo ""
echo "📋 Post-publish checklist:"
echo "  1. Check crates.io pages for all 4 crates"
echo "  2. Verify perl-parser shows 'Tree-sitter compatible' in description"
echo "  3. Verify perl-parser-pest shows 'legacy' warning"
echo "  4. Test LSP installation: cargo install perl-parser --bin perl-lsp --locked"
echo "  5. Create GitHub release with release notes"
echo ""
echo "🎉 v0.8.3 GA release complete!"