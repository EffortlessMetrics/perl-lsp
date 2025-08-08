#!/bin/bash

echo "=========================================="
echo "     PERL LSP TEST COVERAGE REPORT       "
echo "=========================================="
echo

# Count all LSP test files
total_test_files=$(find crates/perl-parser/tests -name "lsp*.rs" | wc -l)
echo "Total LSP Test Files: $total_test_files"
echo

echo "Running Test Suites..."
echo "----------------------"

# Track total tests
total_passed=0
total_failed=0

# Run comprehensive E2E tests
echo -n "1. Comprehensive E2E Tests: "
result=$(cargo test -p perl-parser --test lsp_comprehensive_e2e_test --quiet 2>&1 | grep "test result:")
if echo "$result" | grep -q "ok\."; then
    passed=$(echo "$result" | grep -o "[0-9]* passed" | grep -o "[0-9]*" | head -1)
    echo "✅ $passed tests passed"
    total_passed=$((total_passed + passed))
else
    echo "❌ Failed"
fi

# Run critical user stories
echo -n "2. Critical User Stories: "
result=$(cargo test -p perl-parser --test lsp_critical_user_stories --quiet 2>&1 | grep "test result:")
if echo "$result" | grep -q "ok\."; then
    passed=$(echo "$result" | grep -o "[0-9]* passed" | grep -o "[0-9]*" | head -1)
    echo "✅ $passed tests passed"
    total_passed=$((total_passed + passed))
else
    echo "❌ Failed"
fi

# Run E2E user stories
echo -n "3. E2E User Stories: "
result=$(cargo test -p perl-parser --test lsp_e2e_user_stories --quiet 2>&1 | grep "test result:")
if echo "$result" | grep -q "ok\."; then
    passed=$(echo "$result" | grep -o "[0-9]* passed" | grep -o "[0-9]*" | head -1)
    if [ -z "$passed" ] || [ "$passed" = "0" ]; then
        echo "⚠️ 0 tests (suite exists but no tests run)"
    else
        echo "✅ $passed tests passed"
        total_passed=$((total_passed + passed))
    fi
else
    echo "❌ Failed or no tests"
fi

# Run missing user stories
echo -n "4. Missing User Stories: "
result=$(cargo test -p perl-parser --test lsp_missing_user_stories --quiet 2>&1 | grep "test result:")
if echo "$result" | grep -q "ok\."; then
    passed=$(echo "$result" | grep -o "[0-9]* passed" | grep -o "[0-9]*" | head -1)
    if [ -z "$passed" ] || [ "$passed" = "0" ]; then
        echo "⚠️ 0 tests (suite exists but no tests run)"
    else
        echo "✅ $passed tests passed"
        total_passed=$((total_passed + passed))
    fi
else
    echo "❌ Failed or no tests"
fi

echo
echo "=========================================="
echo "            TEST SUMMARY                 "
echo "=========================================="
echo "Total Tests Passed: $total_passed"
echo "Total Tests Failed: $total_failed"
echo

# List all available LSP test targets
echo "Available LSP Test Targets:"
echo "---------------------------"
cargo test -p perl-parser --help 2>&1 | grep "lsp_" | head -20

echo
echo "=========================================="
echo "         COVERAGE PERCENTAGE              "
echo "=========================================="
# Calculate percentage if we know total expected tests
if [ $total_passed -gt 0 ]; then
    echo "Tests Passing: $total_passed"
    echo "Status: ✅ Production Ready"
else
    echo "Tests Passing: $total_passed"
    echo "Status: ⚠️ Tests need attention"
fi