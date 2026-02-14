#!/bin/bash

# NodeKind Coverage Analysis Script
# Analyzes which NodeKinds are covered by tests

echo "=== NodeKind Coverage Analysis ==="
echo ""

# First, extract all NodeKinds from the AST definition
echo "1. Extracting all NodeKinds from AST definition..."
ALL_NODEKINDS=$(grep -E "^\s+///?\s*$|^\s+\w+\s*\{" crates/perl-ast/src/ast.rs | grep -B1 "{" | grep -v "^--$" | grep -E "^\s+[A-Z][a-zA-Z]*" | sed 's/^\s*//' | sed 's/\s*{$//' | grep -v "^\s*//" | grep -v "^\s*$" | head -70)

echo "Found NodeKinds:"
echo "$ALL_NODEKINDS"
echo ""

# Count total NodeKinds
TOTAL_COUNT=$(echo "$ALL_NODEKINDS" | wc -l)
echo "Total NodeKinds: $TOTAL_COUNT"
echo ""

# Check which NodeKinds appear in test files
echo "2. Checking NodeKind coverage in test files..."

# Create a temporary file for covered NodeKinds
COVERED_FILE=$(mktemp)
echo "" > $COVERED_FILE

# Search for NodeKind references in test files
for kind in $ALL_NODEKINDS; do
    if grep -r "NodeKind::$kind" crates/*/tests/ test_corpus/ > /dev/null 2>&1; then
        echo "$kind" >> $COVERED_FILE
    fi
done

COVERED_COUNT=$(cat $COVERED_FILE | wc -l)
echo "Covered NodeKinds: $COVERED_COUNT"

# Calculate coverage percentage
if [ $TOTAL_COUNT -gt 0 ]; then
    COVERAGE=$(echo "scale=2; $COVERED_COUNT * 100 / $TOTAL_COUNT" | bc)
    echo "Coverage: $COVERAGE%"
fi

echo ""
echo "3. NodeKinds with test coverage:"
cat $COVERED_FILE | sort

echo ""
echo "4. NodeKinds WITHOUT test coverage:"
for kind in $ALL_NODEKINDS; do
    if ! grep -q "^$kind$" $COVERED_FILE; then
        echo "$kind"
    fi
done

# Clean up
rm -f $COVERED_FILE

echo ""
echo "5. Analyzing test corpus files..."
echo "Test corpus files that might test NodeKinds:"
ls -la test_corpus/*.pl | head -20

echo ""
echo "6. Checking for specific NodeKind patterns in corpus..."
for kind in Format Glob Tie Sigil; do
    echo -n "$kind: "
    if grep -r "$kind" test_corpus/ > /dev/null 2>&1; then
        echo "Found in corpus"
    else
        echo "Not found in corpus"
    fi
done

echo ""
echo "=== Analysis Complete ==="