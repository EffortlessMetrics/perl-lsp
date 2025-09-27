#!/bin/bash

# Test Validation Script - Ensures test quality and completeness
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=========================================="
echo "    PERL LSP TEST VALIDATION SUITE"
echo "=========================================="
echo ""

# Track overall status
FAILED=0

# 1. Check for tautological assertions
echo "1. Checking for tautological assertions..."
TAUTOLOGIES=0

# Check for always-true assertions
if grep -r "assert!(.*\.is_some() || .*\.is_none())" tests/ 2>/dev/null; then
    echo -e "${RED}❌ Found tautological assertions (is_some || is_none)${NC}"
    TAUTOLOGIES=$((TAUTOLOGIES + 1))
fi

if grep -r "assert!(true)" tests/ 2>/dev/null; then
    echo -e "${RED}❌ Found assert!(true)${NC}"
    TAUTOLOGIES=$((TAUTOLOGIES + 1))
fi

if grep -r "assert!(.*\.len() >= 0)" tests/ 2>/dev/null; then
    echo -e "${RED}❌ Found tautological length check (>= 0)${NC}"
    TAUTOLOGIES=$((TAUTOLOGIES + 1))
fi

if grep -r "assert!(.*\.is_ok() || .*\.is_err())" tests/ 2>/dev/null; then
    echo -e "${RED}❌ Found tautological Result check${NC}"
    TAUTOLOGIES=$((TAUTOLOGIES + 1))
fi

if [ $TAUTOLOGIES -eq 0 ]; then
    echo -e "${GREEN}✅ No tautological assertions found${NC}"
else
    FAILED=$((FAILED + 1))
fi

echo ""

# 2. Check for proper assertion usage
echo "2. Validating assertion patterns..."
ASSERTION_ISSUES=0

# Check for unwrap without context
if grep -r "\.unwrap()" tests/ | grep -v "// Safe:" | grep -v "// SAFETY:" 2>/dev/null; then
    echo -e "${YELLOW}⚠️  Found unwrap() without safety comment${NC}"
    ASSERTION_ISSUES=$((ASSERTION_ISSUES + 1))
fi

# Check for expect with generic messages
if grep -r '\.expect("failed")' tests/ 2>/dev/null; then
    echo -e "${YELLOW}⚠️  Found expect() with generic message${NC}"
    ASSERTION_ISSUES=$((ASSERTION_ISSUES + 1))
fi

if [ $ASSERTION_ISSUES -eq 0 ]; then
    echo -e "${GREEN}✅ Assertion patterns look good${NC}"
fi

echo ""

# 3. Check test coverage metrics
echo "3. Analyzing test coverage..."

# Count test functions
TOTAL_TESTS=$(find tests -name "*.rs" -exec grep -c "#\[test\]" {} \; 2>/dev/null | paste -sd+ | bc)
echo "   Total test functions: $TOTAL_TESTS"

# Count assertions
TOTAL_ASSERTIONS=$(grep -r "assert" tests/ 2>/dev/null | wc -l)
echo "   Total assertions: $TOTAL_ASSERTIONS"

# Calculate assertion density
if [ $TOTAL_TESTS -gt 0 ]; then
    ASSERTIONS_PER_TEST=$((TOTAL_ASSERTIONS / TOTAL_TESTS))
    echo "   Assertions per test: ~$ASSERTIONS_PER_TEST"
    
    if [ $ASSERTIONS_PER_TEST -lt 2 ]; then
        echo -e "${YELLOW}⚠️  Low assertion density (< 2 per test)${NC}"
    else
        echo -e "${GREEN}✅ Good assertion density${NC}"
    fi
fi

echo ""

# 4. Check for test organization
echo "4. Checking test organization..."
ORG_ISSUES=0

# Check for test modules
if ! grep -r "#\[cfg(test)\]" crates/perl-parser/src/ 2>/dev/null | head -1 > /dev/null; then
    echo -e "${YELLOW}⚠️  No unit test modules found in src/${NC}"
    ORG_ISSUES=$((ORG_ISSUES + 1))
fi

# Check for test documentation
if ! grep -r "///" tests/ 2>/dev/null | head -1 > /dev/null; then
    echo -e "${YELLOW}⚠️  No documentation comments in tests${NC}"
    ORG_ISSUES=$((ORG_ISSUES + 1))
fi

if [ $ORG_ISSUES -eq 0 ]; then
    echo -e "${GREEN}✅ Tests are well organized${NC}"
fi

echo ""

# 5. Run actual tests with detailed output
echo "5. Running test suites..."

# Function to run test and capture results
run_test_suite() {
    local suite=$1
    local name=$2
    
    echo -n "   $name: "
    
    if output=$(cargo test -p perl-parser --test $suite --quiet 2>&1); then
        if echo "$output" | grep -q "test result: ok"; then
            passed=$(echo "$output" | grep -o "[0-9]* passed" | grep -o "[0-9]*" | head -1)
            echo -e "${GREEN}✅ $passed tests passed${NC}"
            return 0
        fi
    fi
    
    echo -e "${RED}❌ Failed${NC}"
    return 1
}

# Run each test suite
TEST_FAILURES=0

run_test_suite "lsp_comprehensive_e2e_test" "Comprehensive E2E" || TEST_FAILURES=$((TEST_FAILURES + 1))
run_test_suite "lsp_critical_user_stories" "Critical Stories" || TEST_FAILURES=$((TEST_FAILURES + 1))
run_test_suite "lsp_e2e_user_stories" "E2E Stories" || TEST_FAILURES=$((TEST_FAILURES + 1))
run_test_suite "lsp_missing_user_stories" "Missing Stories" || TEST_FAILURES=$((TEST_FAILURES + 1))

if [ $TEST_FAILURES -gt 0 ]; then
    FAILED=$((FAILED + 1))
fi

echo ""

# 6. Check for flaky tests (run multiple times)
echo "6. Checking for flaky tests..."
echo -n "   Running tests 3 times... "

FLAKY=0
for i in {1..3}; do
    if ! cargo test -p perl-parser --quiet 2>&1 | grep -q "test result: ok"; then
        FLAKY=$((FLAKY + 1))
    fi
done

if [ $FLAKY -eq 0 ]; then
    echo -e "${GREEN}✅ No flaky tests detected${NC}"
else
    echo -e "${RED}❌ Tests failed $FLAKY/3 times (possible flakiness)${NC}"
    FAILED=$((FAILED + 1))
fi

echo ""

# 7. Check helper function usage
echo "7. Validating test helper usage..."

# Count custom assertion usage
CUSTOM_ASSERTIONS=$(grep -r "assert_.*(" tests/ | grep -v "assert!(" | grep -v "assert_eq!(" | grep -v "assert_ne!(" | wc -l)
echo "   Custom assertion calls: $CUSTOM_ASSERTIONS"

if [ $CUSTOM_ASSERTIONS -gt 20 ]; then
    echo -e "${GREEN}✅ Good use of custom assertions${NC}"
elif [ $CUSTOM_ASSERTIONS -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Limited use of custom assertions${NC}"
else
    echo -e "${RED}❌ No custom assertions found${NC}"
fi

echo ""

# 8. Performance check
echo "8. Testing performance..."
echo -n "   Running full test suite with timing... "

START_TIME=$(date +%s)
if cargo test -p perl-parser --quiet 2>&1 > /dev/null; then
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    
    if [ $DURATION -lt 5 ]; then
        echo -e "${GREEN}✅ Fast execution (${DURATION}s)${NC}"
    elif [ $DURATION -lt 10 ]; then
        echo -e "${YELLOW}⚠️  Moderate execution time (${DURATION}s)${NC}"
    else
        echo -e "${RED}❌ Slow execution (${DURATION}s)${NC}"
    fi
fi

echo ""

# Final summary
echo "=========================================="
echo "              SUMMARY"
echo "=========================================="

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ ALL VALIDATIONS PASSED${NC}"
    echo ""
    echo "Test Quality Metrics:"
    echo "  • $TOTAL_TESTS test functions"
    echo "  • $TOTAL_ASSERTIONS assertions"
    echo "  • ~$ASSERTIONS_PER_TEST assertions per test"
    echo "  • $CUSTOM_ASSERTIONS custom assertion calls"
    echo "  • All tests deterministic"
    echo "  • No tautological patterns"
    exit 0
else
    echo -e "${RED}❌ $FAILED VALIDATION(S) FAILED${NC}"
    echo ""
    echo "Please review and fix the issues above."
    exit 1
fi