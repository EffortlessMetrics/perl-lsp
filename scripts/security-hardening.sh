#!/bin/bash

# Production Hardening - Security Script
# This script implements comprehensive security hardening for Phase 6

set -euo pipefail

echo "🔒 Production Hardening - Security Script"
echo "=========================================="

# Colors for output
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
        "OK") echo -e "${GREEN}✅ $message${NC}" ;;
        "WARN") echo -e "${YELLOW}⚠️  $message${NC}" ;;
        "ERROR") echo -e "${RED}❌ $message${NC}" ;;
        "INFO") echo -e "${BLUE}ℹ️  $message${NC}" ;;
    esac
}

# 1. Dependency Vulnerability Scanning
echo ""
echo "1. Dependency Vulnerability Scanning"
echo "----------------------------------"

# Check if cargo-audit is installed
if ! command -v cargo-audit &> /dev/null; then
    print_status "WARN" "cargo-audit not found, installing..."
    cargo install cargo-audit --locked
fi

# Run security audit
print_status "INFO" "Running cargo audit..."
if cargo audit 2>/dev/null; then
    print_status "OK" "No critical vulnerabilities found"
else
    print_status "WARN" "Some vulnerabilities detected - review output above"
fi

# Check if cargo-deny is installed
if ! command -v cargo-deny &> /dev/null; then
    print_status "WARN" "cargo-deny not found, installing..."
    cargo install cargo-deny --locked
fi

# Run cargo-deny check
print_status "INFO" "Running cargo deny checks..."
if cargo deny check 2>/dev/null; then
    print_status "OK" "All dependency policies passed"
else
    print_status "WARN" "Some dependency policy violations - review output above"
fi

# 2. Input Validation Analysis
echo ""
echo "2. Input Validation Analysis"
echo "----------------------------"

# Find potential unsafe input handling
print_status "INFO" "Scanning for unsafe input handling..."

UNSAFE_PATTERNS=(
    "unwrap()"
    "expect("
    "panic!"
    "from_utf8_unchecked"
    "from_str_unchecked"
    "slice_unchecked"
    "get_unchecked"
)

TOTAL_ISSUES=0
for pattern in "${UNSAFE_PATTERNS[@]}"; do
    count=$(find crates -name "*.rs" -exec grep -l "$pattern" {} \; | wc -l)
    if [ "$count" -gt 0 ]; then
        print_status "WARN" "Found $count files with $pattern"
        TOTAL_ISSUES=$((TOTAL_ISSUES + count))
    fi
done

if [ "$TOTAL_ISSUES" -eq 0 ]; then
    print_status "OK" "No obvious unsafe patterns found"
else
    print_status "WARN" "Found $TOTAL_ISSUES potential unsafe patterns"
fi

# 3. Path Traversal Prevention Check
echo ""
echo "3. Path Traversal Prevention Check"
echo "----------------------------------"

# Look for file path operations that might be vulnerable
PATH_PATTERNS=(
    "std::fs::"
    "std::path::Path"
    "open("
    "read("
    "write("
)

print_status "INFO" "Checking path handling safety..."
PATH_FILES=$(find crates -name "*.rs" -exec grep -l "std::fs::\|std::path::" {} \; | wc -l)
print_status "INFO" "Found $PATH_FILES files with file system operations"

# Check for canonicalization usage
CANONICALIZED=$(find crates -name "*.rs" -exec grep -l "canonicalize\|components\|parent" {} \; | wc -l)
if [ "$CANONICALIZED" -gt 0 ]; then
    print_status "OK" "Found $CANONICALIZED files using path safety functions"
else
    print_status "WARN" "Few files using path safety functions found"
fi

# 4. Process Isolation Analysis
echo ""
echo "4. Process Isolation Analysis"
echo "-----------------------------"

# Check for process spawning
PROCESS_PATTERNS=(
    "Command::"
    "std::process::"
    "exec("
    "spawn("
)

print_status "INFO" "Checking process isolation..."
PROCESS_FILES=$(find crates -name "*.rs" -exec grep -l "Command::\|std::process::" {} \; | wc -l)
if [ "$PROCESS_FILES" -gt 0 ]; then
    print_status "INFO" "Found $PROCESS_FILES files with process operations"
    
    # Check for sandboxing patterns
    SANDBOXED=$(find crates -name "*.rs" -exec grep -l "chroot\|namespace\|seccomp\|sandbox" {} \; | wc -l)
    if [ "$SANDBOXED" -gt 0 ]; then
        print_status "OK" "Found $SANDBOXED files with sandboxing patterns"
    else
        print_status "WARN" "Process operations found but limited sandboxing detected"
    fi
else
    print_status "OK" "No process spawning operations found"
fi

# 5. Memory Safety Check
echo ""
echo "5. Memory Safety Check"
echo "----------------------"

# Look for unsafe blocks
UNSAFE_BLOCKS=$(find crates -name "*.rs" -exec grep -c "unsafe {" {} \; | awk '{sum += $1} END {print sum}')
print_status "INFO" "Found $UNSAFE_BLOCKS unsafe blocks"

if [ "$UNSAFE_BLOCKS" -gt 0 ]; then
    # Check for safety documentation
    DOCUMENTED_UNSAFE=$(find crates -name "*.rs" -exec grep -l "Safety:\|# Safety" {} \; | wc -l)
    print_status "INFO" "Found $DOCUMENTED_UNSAFE files with safety documentation"
    
    if [ "$DOCUMENTED_UNSAFE" -ge "$UNSAFE_BLOCKS" ]; then
        print_status "OK" "All unsafe blocks appear to be documented"
    else
        print_status "WARN" "Some unsafe blocks may lack safety documentation"
    fi
else
    print_status "OK" "No unsafe blocks found"
fi

# 6. Configuration Security
echo ""
echo "6. Configuration Security"
echo "-------------------------"

# Check for hardcoded secrets
SECRET_PATTERNS=(
    "password\s*="
    "api_key\s*="
    "secret\s*="
    "token\s*="
    "private_key"
)

SECRETS_FOUND=0
for pattern in "${SECRET_PATTERNS[@]}"; do
    count=$(find crates -name "*.rs" -exec grep -i "$pattern" {} \; | wc -l)
    if [ "$count" -gt 0 ]; then
        print_status "WARN" "Found $count potential hardcoded secrets with pattern: $pattern"
        SECRETS_FOUND=$((SECRETS_FOUND + count))
    fi
done

if [ "$SECRETS_FOUND" -eq 0 ]; then
    print_status "OK" "No obvious hardcoded secrets found"
else
    print_status "WARN" "Found $SECRETS_FOUND potential hardcoded secrets"
fi

# 7. Security Testing Coverage
echo ""
echo "7. Security Testing Coverage"
echo "---------------------------"

# Count security-related tests
SECURITY_TESTS=$(find crates -name "*security*" -o -name "*vulnerab*" -o -name "*audit*" | wc -l)
print_status "INFO" "Found $SECURITY_TESTS security-related test files"

# Check for fuzz tests
FUZZ_TESTS=$(find . -name "fuzz" -type d | wc -l)
print_status "INFO" "Found $FUZZ_TESTS fuzz test directories"

# 8. Generate Security Report
echo ""
echo "8. Security Report Generation"
echo "----------------------------"

REPORT_FILE="security_hardening_report_$(date +%Y%m%d_%H%M%S).json"

cat > "$REPORT_FILE" << EOF
{
  "timestamp": "$(date -Iseconds)",
  "scan_type": "production_hardening_phase6",
  "results": {
    "vulnerability_scan": {
      "cargo_audit": "completed",
      "cargo_deny": "completed"
    },
    "input_validation": {
      "unsafe_patterns_found": $TOTAL_ISSUES,
      "files_scanned": $(find crates -name "*.rs" | wc -l)
    },
    "path_traversal": {
      "fs_operations_files": $PATH_FILES,
      "path_safety_files": $CANONICALIZED
    },
    "process_isolation": {
      "process_operations_files": $PROCESS_FILES,
      "sandboxing_files": $SANDBOXED
    },
    "memory_safety": {
      "unsafe_blocks": $UNSAFE_BLOCKS,
      "documented_unsafe": $DOCUMENTED_UNSAFE
    },
    "configuration_security": {
      "potential_secrets": $SECRETS_FOUND
    },
    "testing_coverage": {
      "security_test_files": $SECURITY_TESTS,
      "fuzz_directories": $FUZZ_TESTS
    }
  }
}
EOF

print_status "OK" "Security report generated: $REPORT_FILE"

# 9. Summary and Recommendations
echo ""
echo "9. Summary and Recommendations"
echo "=============================="

print_status "INFO" "Security hardening scan completed"
print_status "INFO" "Review the detailed output above for specific issues"

# Provide recommendations based on findings
if [ "$TOTAL_ISSUES" -gt 0 ]; then
    print_status "WARN" "Recommendation: Review and fix unsafe input handling patterns"
fi

if [ "$SECRETS_FOUND" -gt 0 ]; then
    print_status "WARN" "Recommendation: Remove hardcoded secrets and use secure configuration"
fi

if [ "$UNSAFE_BLOCKS" -gt "$DOCUMENTED_UNSAFE" ]; then
    print_status "WARN" "Recommendation: Add safety documentation for all unsafe blocks"
fi

if [ "$PROCESS_FILES" -gt 0 ] && [ "$SANDBOXED" -eq 0 ]; then
    print_status "WARN" "Recommendation: Implement process isolation for process operations"
fi

print_status "OK" "Production hardening security script completed successfully"

echo ""
echo "Next steps:"
echo "1. Review and address any WARNINGS above"
echo "2. Run comprehensive tests: just test-full"
echo "3. Verify production gates: just merge-gate"
echo "4. Check security report: $REPORT_FILE"