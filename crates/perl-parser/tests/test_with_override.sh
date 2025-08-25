#!/bin/bash
# Test script demonstrating feature catalog override

set -e

echo "Testing with minimal features catalog..."
echo "This disables several normally-advertised features"
echo

# Run with override catalog
FEATURES_TOML_OVERRIDE=crates/perl-parser/tests/data/features_minimal.toml \
    cargo test -p perl-parser --test lsp_feature_gating_test -- --nocapture

echo
echo "Testing with disabled features catalog..."
FEATURES_TOML_OVERRIDE=crates/perl-parser/tests/data/features_disabled_test.toml \
    cargo test -p perl-parser --test lsp_features_snapshot_test -- --nocapture

echo
echo "✅ Override testing complete!"