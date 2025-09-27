#!/bin/bash
# Release script for tree-sitter-perl workspace
# Usage: ./release.sh [version]

set -e

VERSION="${1:-0.8.5}"
DATE=$(date +%Y-%m-%d)

echo "🚀 Preparing release v$VERSION"
echo "================================"

# 1. Clean build
echo "📦 Clean building workspace..."
cargo clean
cargo build --workspace --release

# 2. Format check
echo "🎨 Checking formatting..."
cargo fmt --all -- --check

# 3. Clippy check (allow warnings for now)
echo "🔍 Running clippy..."
cargo clippy --workspace

# 4. Run tests
echo "🧪 Running tests..."
cargo test --workspace

# 5. Check capability snapshot
echo "📸 Verifying capability snapshot..."
cargo test -p perl-parser --test lsp_capabilities_snapshot test_production_capabilities_snapshot

# 6. Build benchmarks
echo "⚡ Building benchmarks..."
cargo build -p parser-benchmarks --benches

# 7. Final safety checks
echo "🔒 Running safety checks..."
DEBUG_COUNT=$(grep -rn "DEBUG:" crates/perl-parser/src | wc -l)
if [ "$DEBUG_COUNT" -ne "0" ]; then
    echo "❌ Found DEBUG markers in code!"
    exit 1
fi

echo ""
echo "✅ All checks passed!"
echo ""
echo "📋 Release checklist:"
echo "  [ ] Update version in Cargo.toml files to $VERSION"
echo "  [ ] Update CHANGELOG.md date from 2025-02-XX to $DATE"
echo "  [ ] Commit changes: git commit -am \"chore: release v$VERSION\""
echo "  [ ] Tag release: git tag -a v$VERSION -m \"Release v$VERSION\""
echo "  [ ] Push: git push && git push --tags"
echo "  [ ] Publish crates in order:"
echo "      cargo publish -p perl-lexer"
echo "      cargo publish -p perl-corpus"
echo "      cargo publish -p perl-parser"
echo "  [ ] Create GitHub release with binaries"
echo ""
echo "📝 Changelog highlights for v$VERSION:"
echo "  - Stable diagnostic codes (PL001-PL499)"
echo "  - Pull diagnostics support with automatic suppression"
echo "  - Typed capabilities with snapshot testing"
echo "  - Enhanced inlay hints with smart anchoring"
echo "  - Consolidated builtin signatures with phf"