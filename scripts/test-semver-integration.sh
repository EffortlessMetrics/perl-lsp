#!/usr/bin/env bash
# Test script for SemVer breaking change detection integration
# Issue #277

set -euo pipefail

echo "================================="
echo "SemVer Integration Test"
echo "================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0

# Test helper
test_command() {
    local name="$1"
    local command="$2"
    TESTS_RUN=$((TESTS_RUN + 1))

    echo -n "Test $TESTS_RUN: $name... "

    if eval "$command" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        echo -e "${RED}✗${NC}"
        return 1
    fi
}

# Test 1: cargo-semver-checks is available
test_command "cargo-semver-checks installed" "command -v cargo-semver-checks"

# Test 2: .cargo-semver-checks.toml exists
test_command "Configuration file exists" "test -f .cargo-semver-checks.toml"

# Test 3: justfile recipes exist
test_command "just semver-check recipe" "just --list | grep -q semver-check"
test_command "just semver-check-package recipe" "just --list | grep -q semver-check-package"
test_command "just semver-list-baselines recipe" "just --list | grep -q semver-list-baselines"

# Test 4: Baseline tags available
test_command "Baseline tags exist" "git tag | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | wc -l | grep -qv '^0$'"

# Test 5: Can list baselines
echo -n "Test $((TESTS_RUN + 1)): Can list baselines... "
TESTS_RUN=$((TESTS_RUN + 1))
BASELINE_OUTPUT=$(just semver-list-baselines 2>&1)
if echo "$BASELINE_OUTPUT" | grep -q "v0.8.5"; then
    echo -e "${GREEN}✓${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    echo -e "${RED}✗${NC}"
    echo "  Expected v0.8.5 in output, got:"
    echo "  $BASELINE_OUTPUT"
fi

# Test 6: CI workflow includes semver-check job
test_command "CI workflow has semver-check job" "grep -q 'semver-check:' .github/workflows/quality-checks.yml"

# Test 7: CONTRIBUTING.md documents SemVer workflow
test_command "CONTRIBUTING.md mentions SemVer" "grep -q 'ci:semver' CONTRIBUTING.md"

# Test 8: Documentation exists
test_command "SEMVER_WORKFLOW.md exists" "test -f docs/SEMVER_WORKFLOW.md"

echo ""
echo "================================="
echo "Test Summary"
echo "================================="
echo "Tests run: $TESTS_RUN"
echo "Tests passed: $TESTS_PASSED"
echo "Tests failed: $((TESTS_RUN - TESTS_PASSED))"
echo ""

if [ $TESTS_PASSED -eq $TESTS_RUN ]; then
    echo -e "${GREEN}✅ All tests passed!${NC}"
    echo ""
    echo "SemVer breaking change detection is properly configured."
    echo ""
    echo "Usage:"
    echo "  just semver-check                    # Check all packages"
    echo "  just semver-check-package perl-parser  # Check specific package"
    echo "  just semver-list-baselines           # List available baselines"
    echo ""
    echo "For more information, see:"
    echo "  - docs/SEMVER_WORKFLOW.md"
    echo "  - CONTRIBUTING.md (SemVer section)"
    exit 0
else
    echo -e "${RED}❌ Some tests failed.${NC}"
    echo ""
    echo "Please review the failures above and ensure:"
    echo "  1. cargo-semver-checks is installed: cargo install cargo-semver-checks --locked"
    echo "  2. All configuration files are in place"
    echo "  3. CI workflow is updated"
    exit 1
fi
