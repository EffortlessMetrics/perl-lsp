#!/bin/bash
set -e

echo "🔍 Smoke testing v0.8.3 release..."

# Create temp workspace
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

echo "📦 Testing perl-parser installation..."
cargo new test-perl-parser
cd test-perl-parser

# Test adding perl-parser
echo "Testing cargo add perl-parser@0.8.3 --dry-run..."
cargo add perl-parser@0.8.3 --dry-run

# Test basic usage would work
cat > src/main.rs << 'RUST'
use perl_parser::Parser;

fn main() {
    let mut parser = Parser::new();
    let code = "print 'Hello, World!'";
    match parser.parse(code) {
        Ok(node) => println!("Parsed successfully: {}", node.to_sexp()),
        Err(e) => eprintln!("Parse error: {}", e),
    }
}
RUST

cargo check

echo "✅ perl-parser smoke test passed!"

# Test LSP binary
echo "🔧 Testing LSP installation..."
cargo install perl-parser --bin perl-lsp --dry-run

echo "✅ All smoke tests passed!"
echo "📍 Test location: $TEMP_DIR"