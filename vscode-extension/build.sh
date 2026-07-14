#!/bin/bash

echo "Building VSCode Perl Language Server Extension..."

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    echo "Error: npm is not installed. Please install Node.js and npm."
    exit 1
fi

# Verify the declared toolchain before installing dependencies.
echo "Verifying toolchain..."
npm run doctor

# Install the locked dependencies
echo "Installing dependencies..."
npm ci

# Compile TypeScript
echo "Compiling TypeScript..."
npm run compile

# Bundle LSP binary
echo "Bundling perllsp binary..."
npm run bundle-lsp

# Package extension
echo "Packaging extension..."
npm run package

echo "Build complete! Extension packaged as perl-lsp-rs-*.vsix"
echo ""
echo "To install locally:"
echo "  code --install-extension perl-lsp-rs-*.vsix"
echo ""
echo "To publish to marketplace:"
echo "  npm run publish"
