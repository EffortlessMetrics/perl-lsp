#!/bin/bash
# CI script to test all heredoc features and ensure no regression
# This script tests all the heredoc improvements:
# - Multi-line statement heredocs
# - Statement boundary tracking
# - Builtin list operators (print, say, warn, die)

set -e

echo "🧪 Running comprehensive heredoc tests..."

# Run using xtask if available
if command -v cargo xtask &> /dev/null; then
    echo "Using cargo xtask..."
    cargo xtask test-heredoc --release
else
    echo "Running tests directly..."
    # Run each heredoc test suite
    cargo test --features pure-rust --release --test heredoc_missing_features_tests
    cargo test --features pure-rust --release --test heredoc_integration_tests
    cargo test --features pure-rust --release --test comprehensive_heredoc_tests
fi

echo "✅ All heredoc tests passed!"