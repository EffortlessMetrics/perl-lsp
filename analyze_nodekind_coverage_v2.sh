#!/bin/bash

# NodeKind Coverage Analysis Script v2
# More accurate extraction of NodeKinds from AST definition

echo "=== NodeKind Coverage Analysis v2 ==="
echo ""

# Extract NodeKind variants more accurately
echo "1. Extracting all NodeKinds from AST definition..."
ALL_NODEKINDS=$(grep -E "^\s+///?\s*$|^\s+\w+\s*\{" crates/perl-ast/src/ast.rs | grep -B1 "{" | grep -v "^--$" | grep -E "^\s+[A-Z][a-zA-Z]*" | sed 's/^\s*//' | sed 's/\s*{$//' | grep -v "^\s*//" | grep -v "^\s*$" | grep -v "^Node\s*" | grep -v "^\s*{" | grep -v "^\s*kind," | grep -v "^\s*location" | head -70)

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
    if grep -r "NodeKind::$kind" crates/*/tests/ > /dev/null 2>&1; then
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

# Check coverage in corpus files
echo ""
echo "5. Checking NodeKind coverage in test corpus..."
CORPUS_COVERED_FILE=$(mktemp)
echo "" > $CORPUS_COVERED_FILE

for kind in $ALL_NODEKINDS; do
    # Check if the NodeKind is mentioned in corpus files or if corpus files test the feature
    case $kind in
        "Readline")
            if grep -r "<.*>" test_corpus/ | grep -v "<<" > /dev/null 2>&1; then
                echo "$kind" >> $CORPUS_COVERED_FILE
            fi
            ;;
        "Glob")
            if grep -r "<.*\*.*>" test_corpus/ > /dev/null 2>&1; then
                echo "$kind" >> $CORPUS_COVERED_FILE
            fi
            ;;
        "Try")
            if grep -r "try\|catch" test_corpus/ -i > /dev/null 2>&1; then
                echo "$kind" >> $CORPUS_COVERED_FILE
            fi
            ;;
        "Given"|"When"|"Default")
            if grep -r "given\|when\|default" test_corpus/ -i > /dev/null 2>&1; then
                echo "$kind" >> $CORPUS_COVERED_FILE
            fi
            ;;
        "Package")
            if grep -r "package " test_corpus/ > /dev/null 2>&1; then
                echo "$kind" >> $CORPUS_COVERED_FILE
            fi
            ;;
        "Class")
            if grep -r "class " test_corpus/ -i > /dev/null 2>&1; then
                echo "$kind" >> $CORPUS_COVERED_FILE
            fi
            ;;
        "Method")
            if grep -r "method" test_corpus/ -i > /dev/null 2>&1; then
                echo "$kind" >> $CORPUS_COVERED_FILE
            fi
            ;;
        "Signature"|"Prototype")
            if grep -r "sub.*[($@%]" test_corpus/ > /dev/null 2>&1; then
                echo "$kind" >> $CORPUS_COVERED_FILE
            fi
            ;;
        "DataSection")
            if grep -r "__DATA__\|__END__" test_corpus/ > /dev/null 2>&1; then
                echo "$kind" >> $CORPUS_COVERED_FILE
            fi
            ;;
        *)
            if grep -r "$kind" test_corpus/ -i > /dev/null 2>&1; then
                echo "$kind" >> $CORPUS_COVERED_FILE
            fi
            ;;
    esac
done

CORPUS_COVERED_COUNT=$(cat $CORPUS_COVERED_FILE | wc -l)
echo "NodeKinds covered in corpus: $CORPUS_COVERED_COUNT"
echo ""

echo "NodeKinds with corpus coverage:"
cat $CORPUS_COVERED_FILE | sort

# Clean up
rm -f $COVERED_FILE $CORPUS_COVERED_FILE

echo ""
echo "=== Analysis Complete ==="