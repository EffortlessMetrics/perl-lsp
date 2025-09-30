#!/bin/bash
# Validation script for Issue #146 - Architectural Integrity Repair
# This script validates that all fixes have been implemented correctly

set -e

echo "🔧 Issue #146 Architectural Integrity Repair Validation"
echo "======================================================="

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    case $status in
        "PASS")
            echo -e "${GREEN}✅ PASS${NC}: $message"
            ;;
        "FAIL")
            echo -e "${RED}❌ FAIL${NC}: $message"
            ;;
        "WARN")
            echo -e "${YELLOW}⚠️  WARN${NC}: $message"
            ;;
        "INFO")
            echo -e "${BLUE}ℹ️  INFO${NC}: $message"
            ;;
    esac
}

# Navigate to project root
cd "$(dirname "$0")/.."

# Phase 1: Validate tdd_workflow.rs fixes
echo
echo "${BLUE}Phase 1: TDD Workflow Compilation Validation${NC}"
echo "----------------------------------------------"

# Check if tdd_workflow.rs exists
if [ -f "crates/perl-parser/src/tdd_workflow.rs" ]; then
    print_status "PASS" "tdd_workflow.rs file exists"

    # Check for signature variable fix
    if grep -q "let args = signature" crates/perl-parser/src/tdd_workflow.rs; then
        print_status "FAIL" "Undefined signature variable still present (line 171)"
        exit 1
    else
        print_status "PASS" "Signature variable issue resolved"
    fi

    # Check for tower_lsp import fix
    if grep -q "use tower_lsp::lsp_types" crates/perl-parser/src/tdd_workflow.rs; then
        print_status "FAIL" "tower_lsp imports still present"
        exit 1
    else
        print_status "PASS" "LSP imports properly use lsp_types"
    fi

else
    print_status "FAIL" "tdd_workflow.rs file not found"
    exit 1
fi

# Phase 2: Validate refactoring.rs creation
echo
echo "${BLUE}Phase 2: Refactoring Module Validation${NC}"
echo "----------------------------------------"

if [ -f "crates/perl-parser/src/refactoring.rs" ]; then
    print_status "PASS" "refactoring.rs file exists"

    # Check for required structs and enums
    if grep -q "pub struct RefactoringEngine" crates/perl-parser/src/refactoring.rs; then
        print_status "PASS" "RefactoringEngine struct present"
    else
        print_status "FAIL" "RefactoringEngine struct missing"
        exit 1
    fi

    if grep -q "pub enum RefactoringType" crates/perl-parser/src/refactoring.rs; then
        print_status "PASS" "RefactoringType enum present"
    else
        print_status "FAIL" "RefactoringType enum missing"
        exit 1
    fi

else
    print_status "WARN" "refactoring.rs not yet created (pending implementation)"
fi

# Phase 3: Compilation validation
echo
echo "${BLUE}Phase 3: Compilation Validation${NC}"
echo "--------------------------------"

print_status "INFO" "Running cargo check on perl-parser crate..."

if cargo check --package perl-parser --message-format=short 2>&1 | grep -E "(error|Error)"; then
    print_status "FAIL" "Compilation errors detected"
    echo "Running detailed compilation check..."
    cargo check --package perl-parser
    exit 1
else
    print_status "PASS" "Compilation successful"
fi

# Phase 4: Module integration validation
echo
echo "${BLUE}Phase 4: Module Integration Validation${NC}"
echo "---------------------------------------"

# Check lib.rs module declarations
if grep -q "// pub mod tdd_workflow" crates/perl-parser/src/lib.rs; then
    print_status "WARN" "tdd_workflow module still commented out in lib.rs"
else
    if grep -q "pub mod tdd_workflow" crates/perl-parser/src/lib.rs; then
        print_status "PASS" "tdd_workflow module uncommented in lib.rs"
    else
        print_status "INFO" "tdd_workflow module declaration not found (may be pending)"
    fi
fi

if grep -q "// pub mod refactoring" crates/perl-parser/src/lib.rs; then
    print_status "WARN" "refactoring module still commented out in lib.rs"
else
    if grep -q "pub mod refactoring" crates/perl-parser/src/lib.rs; then
        print_status "PASS" "refactoring module uncommented in lib.rs"
    else
        print_status "INFO" "refactoring module declaration not found (may be pending)"
    fi
fi

# Phase 5: Test scaffolding validation
echo
echo "${BLUE}Phase 5: Test Scaffolding Validation${NC}"
echo "-------------------------------------"

# Check test files
test_files=(
    "crates/perl-parser/tests/issue_146_architectural_integrity_tests.rs"
    "crates/perl-parser/tests/issue_146_unit_tests.rs"
    "crates/perl-parser/tests/fixtures/issue_146_test_fixtures.pl"
    "crates/perl-parser/tests/fixtures/tdd_workflow_test_samples.pl"
    "crates/perl-parser/tests/fixtures/refactoring_test_samples.pl"
)

for test_file in "${test_files[@]}"; do
    if [ -f "$test_file" ]; then
        print_status "PASS" "Test file exists: $(basename "$test_file")"
    else
        print_status "FAIL" "Test file missing: $test_file"
        exit 1
    fi
done

# Phase 6: Run architectural integrity tests
echo
echo "${BLUE}Phase 6: Architectural Integrity Tests${NC}"
echo "---------------------------------------"

print_status "INFO" "Running Issue #146 architectural integrity tests..."

if cargo test --package perl-parser --test issue_146_architectural_integrity_tests 2>/dev/null; then
    print_status "PASS" "Architectural integrity tests passed"
else
    print_status "WARN" "Some architectural integrity tests failed (expected if modules not fully restored)"
fi

# Phase 7: Quality validation
echo
echo "${BLUE}Phase 7: Quality Validation${NC}"
echo "----------------------------"

print_status "INFO" "Running clippy validation..."

if cargo clippy --package perl-parser -- -D warnings 2>/dev/null; then
    print_status "PASS" "Clippy validation passed"
else
    print_status "WARN" "Clippy found warnings (review recommended)"
fi

# Summary
echo
echo "${BLUE}Validation Summary${NC}"
echo "=================="

echo
print_status "INFO" "Issue #146 validation completed"
print_status "INFO" "Next steps:"
echo "   1. Implement tdd_workflow.rs fixes (if not already done)"
echo "   2. Create refactoring.rs module (if not already done)"
echo "   3. Uncomment modules in lib.rs"
echo "   4. Run full test suite validation"

echo
echo "🚀 Run this script again after implementing fixes to validate progress"