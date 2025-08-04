#!/bin/bash
set -e

# Release script for Perl Language Server v0.6.0
# This script automates the release process

VERSION="0.6.0"
RELEASE_DIR="release"

echo "🚀 Perl Language Server Release Script v$VERSION"
echo "================================================"

# Check if we're on the right branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "master" ] && [ "$CURRENT_BRANCH" != "main" ]; then
    echo "❌ Error: Must be on master/main branch to release"
    echo "   Current branch: $CURRENT_BRANCH"
    exit 1
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo "❌ Error: You have uncommitted changes"
    echo "   Please commit or stash them before releasing"
    exit 1
fi

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check required tools
echo "📋 Checking required tools..."
MISSING_TOOLS=()

if ! command_exists cargo; then
    MISSING_TOOLS+=("cargo")
fi

if ! command_exists npx; then
    MISSING_TOOLS+=("npx")
fi

if ! command_exists strip; then
    MISSING_TOOLS+=("strip (for binary optimization)")
fi

if [ ${#MISSING_TOOLS[@]} -ne 0 ]; then
    echo "❌ Error: Missing required tools: ${MISSING_TOOLS[*]}"
    exit 1
fi

echo "✅ All required tools found"

# Create release directory
echo ""
echo "📁 Creating release directory..."
rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR/binaries"

# Build release binaries
echo ""
echo "🔨 Building release binaries..."

echo "  Building perl-lsp..."
cargo build --release -p perl-parser --bin perl-lsp
cp target/release/perl-lsp "$RELEASE_DIR/binaries/"

echo "  Building perl-dap..."
cargo build --release -p perl-parser --bin perl-dap
cp target/release/perl-dap "$RELEASE_DIR/binaries/"

# Strip binaries for smaller size
echo ""
echo "📦 Optimizing binary sizes..."
ORIGINAL_LSP_SIZE=$(du -h "$RELEASE_DIR/binaries/perl-lsp" | cut -f1)
ORIGINAL_DAP_SIZE=$(du -h "$RELEASE_DIR/binaries/perl-dap" | cut -f1)

strip "$RELEASE_DIR/binaries/perl-lsp"
strip "$RELEASE_DIR/binaries/perl-dap"

NEW_LSP_SIZE=$(du -h "$RELEASE_DIR/binaries/perl-lsp" | cut -f1)
NEW_DAP_SIZE=$(du -h "$RELEASE_DIR/binaries/perl-dap" | cut -f1)

echo "  perl-lsp: $ORIGINAL_LSP_SIZE → $NEW_LSP_SIZE"
echo "  perl-dap: $ORIGINAL_DAP_SIZE → $NEW_DAP_SIZE"

# Package VSCode extension
echo ""
echo "📦 Packaging VSCode extension..."
cd vscode-extension
npm install
npm run compile
npx vsce package
mv perl-language-server-*.vsix "../$RELEASE_DIR/"
cd ..

# Run tests
echo ""
echo "🧪 Running release tests..."
./test_lsp_features.sh > "$RELEASE_DIR/test_results.log" 2>&1
if [ $? -eq 0 ]; then
    echo "✅ Tests passed"
else
    echo "❌ Tests failed. Check $RELEASE_DIR/test_results.log"
    exit 1
fi

# Create checksums
echo ""
echo "🔐 Creating checksums..."
cd "$RELEASE_DIR"
sha256sum binaries/* *.vsix > checksums.txt
cd ..

# Create release tarball
echo ""
echo "📦 Creating release archives..."
cd "$RELEASE_DIR/binaries"
tar -czf "../perl-lsp-$VERSION-linux-x64.tar.gz" perl-lsp perl-dap
cd ../..

# Display release summary
echo ""
echo "✅ Release preparation complete!"
echo "================================================"
echo "Release artifacts in: $RELEASE_DIR/"
echo ""
ls -lh "$RELEASE_DIR/"
echo ""
echo "📋 Next steps:"
echo "1. Review the release artifacts"
echo "2. Create git tag: git tag -a v$VERSION -m 'Release v$VERSION'"
echo "3. Push tag: git push origin v$VERSION"
echo "4. Create GitHub release and upload:"
echo "   - $RELEASE_DIR/perl-lsp-$VERSION-linux-x64.tar.gz"
echo "   - $RELEASE_DIR/perl-language-server-$VERSION.vsix"
echo "   - $RELEASE_DIR/checksums.txt"
echo "5. Publish to crates.io:"
echo "   - cd crates/perl-lexer && cargo publish"
echo "   - cd crates/perl-parser && cargo publish"
echo "6. Publish VSCode extension:"
echo "   - cd vscode-extension && npx vsce publish"
echo ""
echo "🎉 Happy releasing!"