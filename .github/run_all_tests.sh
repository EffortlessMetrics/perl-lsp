#!/bin/bash
# Comprehensive test runner with zero-test detection guard
# Uses --list to verify test binaries contain tests

set -euo pipefail

echo "========================================="
echo "   Perl Parser Comprehensive Test Suite"
echo "========================================="
echo ""

# Enable test-compat feature for old API tests
export CARGO_TEST_FEATURES="--features test-compat"

# Set ci cfg flag when running in CI
if [ "${CI:-false}" = "true" ]; then
    export RUSTFLAGS="${RUSTFLAGS:-} --cfg=ci"
    echo "Running in CI mode - flaky tests will be ignored"
fi

# Run library tests (capture failure to continue with full report)
echo "Running library tests..."
LIB_FAIL=0
if ! cargo test -p perl-parser --lib $CARGO_TEST_FEATURES; then
    echo "⚠️  lib tests had failures; continuing to run all binaries for full report"
    LIB_FAIL=1
fi

echo ""
echo "Running integration tests..."
echo ""

# Build tests without running to get executable paths
echo "Building test executables..."
EXECS=$(cargo test -p perl-parser --no-run --message-format=json $CARGO_TEST_FEATURES 2>/dev/null | \
  jq -r 'select(.reason=="compiler-artifact") | select(.profile.test==true) | .executable // empty' | \
  grep -v '^$' || true)

if [ -z "$EXECS" ]; then
    echo "❌ No test executables found!"
    exit 1
fi

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_FILES=""
ZERO_TEST_FILES=""

# Process executables safely (handles paths with spaces)
while IFS= read -r exe; do
    test_name=$(basename "$exe")
    test_name=${test_name%.exe}  # strip .exe on Windows
    test_name=$(printf "%s" "$test_name" | sed 's/-[[:xdigit:]]\{8,\}$//')
    echo -n "Running $test_name... "
    
    # First verify the test binary has tests using --list
    if ! LIST_OUTPUT=$("$exe" --list --format=terse 2>&1); then
        echo "❌ Failed to list tests"
        FAILED_FILES="$FAILED_FILES $test_name"
        continue
    fi
    
    # Count tests with awk for rock-solid reliability
    # CRLF-safe counting (handles Windows line endings)
    TEST_COUNT=$(
      awk -F': ' '{ x=$NF; sub(/\r$/,"",x); if (x=="test") c++ } END{ print c+0 }' <<< "$LIST_OUTPUT"
    )
    
    if [ "$TEST_COUNT" -eq 0 ]; then
        echo "⚠️  WARNING: 0 tests found!"
        ZERO_TEST_FILES="$ZERO_TEST_FILES $test_name"
        continue
    fi
    
    # Run the actual tests (first attempt, quiet) and capture real exit code
    RUN_OUTPUT=$("$exe" --quiet 2>&1)
    RUN_EXIT=$?
    
    # Check if libtest thought there was a name filter:
    # trigger only when nothing actually ran but some tests were "filtered out"
    if printf "%s\n" "$RUN_OUTPUT" \
         | grep -Eq 'test result: .* 0 passed; 0 failed; 0 ignored; 0 measured; [1-9][0-9]* filtered out;'; then
        echo "ℹ️  Detected libtest filter — re-running each test with --exact"
        status_chunk=0
        tests_passed=0
        tests_failed=0
        
        # Run each test explicitly to bypass any accidental filters
        while IFS= read -r tname; do
            tname=${tname%%: test}  # strip trailing ": test"
            if [ -n "$tname" ]; then
                if "$exe" --exact "$tname" --quiet 2>&1; then
                    tests_passed=$((tests_passed + 1))
                else
                    tests_failed=$((tests_failed + 1))
                    status_chunk=1
                fi
            fi
        done < <( "$exe" --list --format=terse 2>/dev/null \
                  | tr -d '\r' \
                  | awk -F': ' '/: test$/{print $1}' )
        
        if [ $status_chunk -eq 0 ]; then
            echo "✅ $tests_passed tests passed"
            PASSED_TESTS=$((PASSED_TESTS + tests_passed))
            TOTAL_TESTS=$((TOTAL_TESTS + tests_passed))
        else
            echo "❌ $tests_failed tests failed"
            FAILED_FILES="$FAILED_FILES $test_name"
            TOTAL_TESTS=$((TOTAL_TESTS + tests_passed + tests_failed))
            # Re-run failed tests for details
            echo "  Re-running failed tests for details..."
            "$exe" 2>&1 || true
            STATUS=1
        fi
        # Skip the normal path since we already handled it
        continue
    fi
    
    # Normal path: no filter detected
    if [ $RUN_EXIT -eq 0 ]; then
        echo "✅ $TEST_COUNT tests passed"
        PASSED_TESTS=$((PASSED_TESTS + TEST_COUNT))
        TOTAL_TESTS=$((TOTAL_TESTS + TEST_COUNT))
    else
        echo "❌ Some of $TEST_COUNT tests failed"
        # Re-run without --quiet to show details
        echo "  Re-running for details:"
        "$exe" 2>&1 || true
        FAILED_FILES="$FAILED_FILES $test_name"
        TOTAL_TESTS=$((TOTAL_TESTS + TEST_COUNT))
        STATUS=1
    fi
done <<EOF
$EXECS
EOF

echo ""
echo "========================================="
echo "             Test Summary"
echo "========================================="
echo "Total tests discovered: $TOTAL_TESTS"

STATUS=0

if [ -n "$ZERO_TEST_FILES" ]; then
    echo ""
    echo "⚠️  WARNING: Test files with 0 tests (possible regression):"
    for zero_file in $ZERO_TEST_FILES; do
        echo "  - $zero_file"
    done
    echo ""
    echo "This may indicate:"
    echo "  - A wrapper passing stray args (e.g., '2>&1' as argv)"
    echo "  - Missing test functions in the file"
    echo "  - Test discovery issues"
    STATUS=1
fi

if [ -n "$FAILED_FILES" ]; then
    echo ""
    echo "❌ Failed test files:"
    for failed in $FAILED_FILES; do
        echo "  - $failed"
    done
    echo ""
    echo "To debug failures, run:"
    echo "  cargo test -p perl-parser --test <filename> $CARGO_TEST_FEATURES -- --nocapture"
    STATUS=1
fi

if [ $STATUS -eq 0 ] && [ $LIB_FAIL -eq 0 ]; then
    echo ""
    echo "🎉 All $TOTAL_TESTS tests passed successfully!"
else
    echo ""
    echo "❌ Some tests failed. See details above."
    if [ $LIB_FAIL -ne 0 ] || [ $STATUS -ne 0 ]; then
        exit 1
    fi
fi

# Optional: run xtask with a longer timeout, don't fail the whole run if it times out
if [ "${RUN_XTASK:-0}" = "1" ]; then
    echo ""
    echo "========================================="
    echo "Running cargo xtask test (up to 5m)..."
    echo "========================================="
    if timeout 5m cargo xtask test; then
        echo "✅ cargo xtask test done"
    else
        echo "⚠️  cargo xtask test timed out or failed (not marking run failed)"
    fi
fi