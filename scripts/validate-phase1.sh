#!/bin/bash
# Phase 1 LSP Test Stabilization Validation Script
# Verifies that Phase 1 implementation is working correctly

set -e

echo "=== Phase 1 LSP Test Stabilization Validation ==="
echo ""

# Check that nextest config exists
echo "✓ Checking nextest configuration..."
if [ -f ".cargo/nextest.toml" ]; then
    echo "  ✓ .cargo/nextest.toml exists"
else
    echo "  ✗ .cargo/nextest.toml missing"
    exit 1
fi

# Check that stable harness exports exist
echo "✓ Checking stable harness exports..."
if grep -q "spawn_lsp" crates/perl-lsp/tests/support/lsp_harness.rs; then
    echo "  ✓ spawn_lsp() found"
else
    echo "  ✗ spawn_lsp() missing"
    exit 1
fi

if grep -q "handshake_initialize" crates/perl-lsp/tests/support/lsp_harness.rs; then
    echo "  ✓ handshake_initialize() found"
else
    echo "  ✗ handshake_initialize() missing"
    exit 1
fi

if grep -q "pub fn cancel" crates/perl-lsp/tests/support/lsp_harness.rs; then
    echo "  ✓ cancel() method found"
else
    echo "  ✗ cancel() method missing"
    exit 1
fi

if grep -q "pub fn barrier" crates/perl-lsp/tests/support/lsp_harness.rs; then
    echo "  ✓ barrier() method found"
else
    echo "  ✗ barrier() method missing"
    exit 1
fi

# Check that GitHub workflow was updated
echo "✓ Checking GitHub workflow updates..."
if grep -q "nextest" .github/workflows/lsp-tests.yml; then
    echo "  ✓ nextest integration found in workflow"
else
    echo "  ✗ nextest not integrated in workflow"
    exit 1
fi

# Run re-enabled tests
echo "✓ Running re-enabled tests..."
echo "  Testing: test_ga_capabilities_contract"
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_capabilities_contract test_ga_capabilities_contract -- --exact --quiet
echo "  ✓ test_ga_capabilities_contract PASSED"

echo "  Testing: test_cancel_deterministic_stable"
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_cancel_test test_cancel_deterministic_stable -- --exact --quiet
echo "  ✓ test_cancel_deterministic_stable PASSED"

# Check compilation of all LSP tests
echo "✓ Checking LSP test compilation..."
cargo test -p perl-lsp --no-run --quiet 2>&1 | grep -q "Finished" && echo "  ✓ All LSP tests compile successfully"

echo ""
echo "=== Phase 1 Validation Complete ==="
echo ""
echo "Summary:"
echo "  ✓ Stable harness infrastructure in place"
echo "  ✓ Nextest configuration created"
echo "  ✓ GitHub workflows updated"
echo "  ✓ 2 tests re-enabled and passing"
echo "  ✓ Deterministic cancellation patterns working"
echo ""
echo "Next steps:"
echo "  - Phase 2: Port 5-10 more tests to stable harness"
echo "  - Phase 3: Re-enable all ported tests after 10 successful CI runs"
echo ""
