#!/bin/bash

echo "=== Perl Parser Comparison Benchmark ==="
echo "Comparing perl-parser vs tree-sitter-perl-c"
echo ""

# Build both implementations
echo "Building parsers..."
cargo build --release -p perl-parser 2>/dev/null
cargo build --release -p tree-sitter-perl-c 2>/dev/null

# Run the simple comparison benchmark
echo ""
echo "Running benchmarks on standard test cases..."
cargo bench -p parser-benchmarks --bench simple_compare 2>&1 | grep -E "parser-comparison|time:" | grep -B1 "time:"

echo ""
echo "=== Summary ==="
echo "perl-parser: Pure Rust implementation using perl-lexer"
echo "tree-sitter-c: C implementation with tree-sitter"