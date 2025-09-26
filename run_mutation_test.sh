#!/bin/bash

# Manual Mutation Testing Script for Substitution Operator Implementation
# Tests critical logic paths to identify weak spots in test coverage

echo "Manual Mutation Testing for Substitution Operator Implementation"
echo "================================================================"

BACKUP_DIR="/tmp/mutation_backups"
mkdir -p "$BACKUP_DIR"

# Test results tracking
TOTAL_MUTATIONS=0
KILLED_MUTATIONS=0
SURVIVED_MUTATIONS=0
SURVIVORS=()

# Function to backup a file
backup_file() {
    local file="$1"
    local backup_file="$BACKUP_DIR/$(basename "$file").backup"
    cp "$file" "$backup_file"
    echo "$backup_file"
}

# Function to restore a file
restore_file() {
    local file="$1"
    local backup_file="$2"
    cp "$backup_file" "$file"
}

# Function to run substitution tests
run_substitution_tests() {
    cd /home/steven/code/Rust/perl-lsp/review
    timeout 30 cargo test --test substitution_operator_tests 2>&1
}

# Function to test a mutation
test_mutation() {
    local mutation_id="$1"
    local description="$2"
    local file="$3"
    local original="$4"
    local mutated="$5"

    echo ""
    echo "Testing Mutation $mutation_id: $description"
    echo "File: $file"
    echo "Original: $original"
    echo "Mutated: $mutated"

    # Backup original file
    local backup=$(backup_file "$file")

    # Apply mutation
    sed -i "s|$original|$mutated|g" "$file"

    # Run tests
    local test_result=$(run_substitution_tests)
    local exit_code=$?

    # Check if tests passed (mutation survived) or failed (mutation killed)
    if echo "$test_result" | grep -q "test result: ok"; then
        echo "❌ SURVIVED - Tests still pass with mutation"
        SURVIVED_MUTATIONS=$((SURVIVED_MUTATIONS + 1))
        SURVIVORS+=("$mutation_id: $description")
    else
        echo "✅ KILLED - Tests failed with mutation"
        KILLED_MUTATIONS=$((KILLED_MUTATIONS + 1))
    fi

    # Restore original file
    restore_file "$file" "$backup"

    TOTAL_MUTATIONS=$((TOTAL_MUTATIONS + 1))
}

echo "Starting mutation testing..."

# Mutation 1: Change delimiter comparison operator
test_mutation "MUT_001" \
    "Change != to == in is_paired delimiter check" \
    "crates/perl-parser/src/quote_parser.rs" \
    "let is_paired = delimiter != closing;" \
    "let is_paired = delimiter == closing;"

# Mutation 2: Change logical operator in replacement parsing
test_mutation "MUT_002" \
    "Change && to || in replacement parsing condition" \
    "crates/perl-parser/src/quote_parser.rs" \
    "let (replacement, modifiers_str) = if !is_paired && !rest1.is_empty() {" \
    "let (replacement, modifiers_str) = if !is_paired || !rest1.is_empty() {"

# Mutation 3: Change delimiter character check
test_mutation "MUT_003" \
    "Change == to != in closing delimiter check" \
    "crates/perl-parser/src/quote_parser.rs" \
    "c if c == closing => {" \
    "c if c != closing => {"

# Mutation 4: Return early with wrong values
test_mutation "MUT_004" \
    "Return empty values in extract_substitution_parts" \
    "crates/perl-parser/src/quote_parser.rs" \
    "(pattern, replacement, modifiers)" \
    "(String::new(), String::new(), String::new())"

# Mutation 5: Change modifier validation
test_mutation "MUT_005" \
    "Change valid modifier characters in parser_backup.rs" \
    "crates/perl-parser/src/parser_backup.rs" \
    "'g' | 'i' | 'm' | 's' | 'x' | 'o' | 'e' | 'r' => {" \
    "'z' | 'q' | 'w' | 'n' | 'p' | 'k' | 'l' | 'v' => {"

echo ""
echo "================================================================"
echo "MUTATION TESTING REPORT"
echo "================================================================"
echo "Total mutations: $TOTAL_MUTATIONS"
echo "Killed: $KILLED_MUTATIONS"
echo "Survived: $SURVIVED_MUTATIONS"

if [ $TOTAL_MUTATIONS -gt 0 ]; then
    MUTATION_SCORE=$(echo "scale=1; $KILLED_MUTATIONS * 100 / $TOTAL_MUTATIONS" | bc -l)
    echo "Mutation score: ${MUTATION_SCORE}%"
else
    echo "Mutation score: 0%"
fi

echo ""
echo "SURVIVING MUTATIONS:"
if [ ${#SURVIVORS[@]} -eq 0 ]; then
    echo "✅ No surviving mutations - excellent test coverage"
else
    for survivor in "${SURVIVORS[@]}"; do
        echo "❌ $survivor"
    done
fi

echo ""
echo "ROUTING DECISION:"
if [ $SURVIVED_MUTATIONS -eq 0 ]; then
    echo "✅ Perfect mutation score - route to fuzz-tester for next validation phase"
    echo "   The test suite effectively catches all critical mutations in substitution parsing."
elif [ $SURVIVED_MUTATIONS -le 2 ]; then
    echo "🔧 Route to test-hardener - survivors are localizable and targetable"
    echo "   Small number of specific mutations survived, suggesting missing edge case tests."
else
    echo "🎯 Route to fuzz-tester - survivors suggest input-shape gaps"
    echo "   Multiple mutations survived, indicating potential input validation weaknesses."
fi

echo ""
echo "FOCUS AREAS FOR IMPROVEMENT:"
if [ ${#SURVIVORS[@]} -gt 0 ]; then
    echo "- Delimiter handling logic needs stronger test coverage"
    echo "- Edge cases in pattern/replacement extraction need more tests"
    echo "- Modifier validation logic should be tested more thoroughly"
fi

# Clean up backup directory
rm -rf "$BACKUP_DIR"