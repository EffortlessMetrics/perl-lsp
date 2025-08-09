#!/bin/bash
# Comprehensive test runner that works around the test discovery bug
# and ensures all tests run with proper compatibility features

set -e

echo "========================================="
echo "   Perl Parser Comprehensive Test Suite"
echo "========================================="
echo ""

# Enable test-compat feature for old API tests
export CARGO_TEST_FEATURES="--features test-compat"

# Run library tests
echo "Running library tests..."
cargo test -p perl-parser --lib $CARGO_TEST_FEATURES

# Run integration tests with empty filter to work around discovery bug
echo ""
echo "Running integration tests..."
echo "Note: Using empty filter '' to work around test discovery bug"
echo ""

# Get all test files
TEST_FILES=$(ls crates/perl-parser/tests/*.rs 2>/dev/null || true)

if [ -z "$TEST_FILES" ]; then
    echo "No test files found!"
    exit 1
fi

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
FAILED_FILES=""

for test_file in $TEST_FILES; do
    test_name=$(basename "$test_file" .rs)
    echo -n "Running $test_name... "
    
    # Run with empty filter to ensure all tests are discovered
    if cargo test -p perl-parser --test "$test_name" '' $CARGO_TEST_FEATURES --quiet 2>&1; then
        # Count tests that passed
        TEST_COUNT=$(cargo test -p perl-parser --test "$test_name" '' $CARGO_TEST_FEATURES -- --list 2>/dev/null | grep -c ": test" || echo "0")
        echo "✅ $TEST_COUNT tests passed"
        PASSED_TESTS=$((PASSED_TESTS + TEST_COUNT))
        TOTAL_TESTS=$((TOTAL_TESTS + TEST_COUNT))
    else
        # Test failed, get count and mark as failed
        TEST_COUNT=$(cargo test -p perl-parser --test "$test_name" '' $CARGO_TEST_FEATURES -- --list 2>/dev/null | grep -c ": test" || echo "0")
        echo "❌ Some of $TEST_COUNT tests failed"
        FAILED_FILES="$FAILED_FILES $test_name"
        TOTAL_TESTS=$((TOTAL_TESTS + TEST_COUNT))
        # Note: Can't easily count individual test failures without parsing output
    fi
done

echo ""
echo "========================================="
echo "             Test Summary"
echo "========================================="
echo "Total tests discovered: $TOTAL_TESTS"
echo "Status: $(if [ -z "$FAILED_FILES" ]; then echo "✅ All tests passed"; else echo "❌ Some tests failed"; fi)"

if [ -n "$FAILED_FILES" ]; then
    echo ""
    echo "Failed test files:"
    for failed in $FAILED_FILES; do
        echo "  - $failed"
    done
    echo ""
    echo "To debug failures, run:"
    echo "  cargo test -p perl-parser --test <filename> '' --features test-compat -- --nocapture"
    exit 1
fi

echo ""
echo "🎉 All tests passed successfully!"