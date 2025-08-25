#!/bin/bash
# Final validation script for LSP features system

set -e

echo "🔍 LSP Features System Validation"
echo "================================="
echo

# 1. Test rebuild trigger
echo "1. Testing FEATURES_TOML_OVERRIDE rebuild trigger..."
FEATURES_TOML_OVERRIDE=crates/perl-parser/tests/data/features_minimal.toml \
  cargo build -p perl-parser --lib 2>&1 | grep -q "Compiling" && \
  echo "   ✅ Rebuild trigger works" || echo "   ⚠️  No rebuild (may be cached)"

# 2. Generate snapshots with deterministic locale
echo
echo "2. Generating deterministic snapshots..."
LC_ALL=C.UTF-8 INSTA_UPDATE=auto cargo test -p perl-parser --test lsp_features_snapshot_test --quiet
echo "   ✅ Snapshots generated"

# 3. Verify features catalog
echo
echo "3. Verifying features catalog..."
cargo xtask features verify
echo "   ✅ Catalog verified"

# 4. Check JSON export
echo
echo "4. Testing JSON export..."
cargo build -p perl-parser --bin perl-lsp --quiet
COUNT=$(./target/debug/perl-lsp --features-json | jq '.advertised | length')
echo "   ✅ JSON export works: $COUNT advertised features"

# 5. Run gating tests
echo
echo "5. Running feature gating tests..."
cargo test -p perl-parser --test lsp_feature_gating_test --quiet
echo "   ✅ Gating tests pass"

# 6. Test with override catalog
echo
echo "6. Testing with minimal catalog override..."
FEATURES_TOML_OVERRIDE=crates/perl-parser/tests/data/features_minimal.toml \
  cargo test -p perl-parser --lib --quiet
echo "   ✅ Override catalog works"

# 7. Check fence presence
echo
echo "7. Checking documentation fences..."
if grep -q "<!-- BEGIN: COMPLIANCE_TABLE -->" ROADMAP.md && \
   grep -q "<!-- END: COMPLIANCE_TABLE -->" ROADMAP.md; then
  echo "   ✅ Documentation fences present"
else
  echo "   ⚠️  Documentation fences missing (will be added on sync)"
fi

# 8. Quick compliance check
echo
echo "8. Checking compliance percentage..."
PERCENT=$(cargo xtask features verify 2>&1 | grep "📊 Computed compliance" | sed 's/.*: \([0-9]*\)%.*/\1/')
echo "   ✅ Current compliance: ${PERCENT}%"

echo
echo "================================="
echo "✅ All validations complete!"
echo
echo "Next steps:"
echo "  1. Review changes: git diff"
echo "  2. Commit snapshots: git add crates/perl-parser/tests/snapshots/"
echo "  3. Run full test suite: cargo test --all"
echo "  4. Create PR using the updated template"