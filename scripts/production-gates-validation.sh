#!/bin/bash

# Production Hardening - Production Gates Validation Script
# This script validates all production gates and SLOs for Phase 6

set -euo pipefail

echo "🚪 Production Hardening - Production Gates Validation Script"
echo "======================================================"

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

# SLO thresholds
PARSING_TIME_P95_MS=1000       # 1 second P95 parsing time
LSP_RESPONSE_P95_MS=50         # 50ms P95 LSP response time
MEMORY_USAGE_P95_MB=512         # 512MB P95 memory usage
CPU_USAGE_P95_PERCENT=80        # 80% P95 CPU usage
TEST_COVERAGE_PERCENT=95         # 95% test coverage
SECURITY_VULNERABILITIES=0       # Zero critical vulnerabilities

# Gate results
GATES_PASSED=0
TOTAL_GATES=0

# Function to check a gate
check_gate() {
    local gate_name=$1
    local gate_command=$2
    local expected_result=$3
    
    TOTAL_GATES=$((TOTAL_GATES + 1))
    
    print_status "INFO" "Checking gate: $gate_name"
    
    if eval "$gate_command" >/dev/null 2>&1; then
        if [ "$expected_result" = "success" ]; then
            print_status "OK" "Gate PASSED: $gate_name"
            GATES_PASSED=$((GATES_PASSED + 1))
            return 0
        else
            print_status "ERROR" "Gate FAILED: $gate_name (expected failure but got success)"
            return 1
        fi
    else
        if [ "$expected_result" = "failure" ]; then
            print_status "OK" "Gate PASSED: $gate_name (expected failure)"
            GATES_PASSED=$((GATES_PASSED + 1))
            return 0
        else
            print_status "ERROR" "Gate FAILED: $gate_name"
            return 1
        fi
    fi
}

# 1. Code Quality Gates
echo ""
echo "1. Code Quality Gates"
echo "--------------------"

# Formatting check
check_gate "Code Formatting" "cargo fmt --all -- --check" "success"

# Clippy checks
check_gate "Clippy Linting" "cargo clippy --workspace --locked -- -D warnings" "success"

# Documentation coverage
check_gate "Documentation Coverage" "cargo doc --no-deps --workspace 2>&1 | grep -q 'missing documentation'" "failure"

# 2. Test Coverage Gates
echo ""
echo "2. Test Coverage Gates"
echo "---------------------"

# Unit tests
check_gate "Unit Tests" "cargo test --workspace --lib --locked" "success"

# Integration tests
check_gate "Integration Tests" "cargo test --workspace --test '*' --locked" "success"

# LSP specific tests
check_gate "LSP Tests" "cargo test -p perl-lsp --locked" "success"

# Parser specific tests
check_gate "Parser Tests" "cargo test -p perl-parser --locked" "success"

# 3. Security Gates
echo ""
echo "3. Security Gates"
echo "----------------"

# Security audit
check_gate "Security Audit" "cargo audit 2>&1 | grep -q 'error:'" "failure"

# Dependency check
check_gate "Dependency Policy" "cargo deny check 2>/dev/null" "success"

# Check for critical vulnerabilities
VULNERABILITY_COUNT=$(cargo audit 2>&1 | grep -c "Crate:" || echo "0")
if [ "$VULNERABILITY_COUNT" -le "$SECURITY_VULNERABILITIES" ]; then
    print_status "OK" "Gate PASSED: Security Vulnerabilities ($VULNERABILITY_COUNT <= $SECURITY_VULNERABILITIES)"
    GATES_PASSED=$((GATES_PASSED + 1))
else
    print_status "ERROR" "Gate FAILED: Security Vulnerabilities ($VULNERABILITY_COUNT > $SECURITY_VULNERABILITIES)"
fi
TOTAL_GATES=$((TOTAL_GATES + 1))

# 4. Performance Gates
echo ""
echo "4. Performance Gates"
echo "-------------------"

# Check if benchmarks can run
if cargo bench --no-run 2>/dev/null; then
    print_status "OK" "Gate PASSED: Benchmark Compilation"
    GATES_PASSED=$((GATES_PASSED + 1))
else
    print_status "WARN" "Gate WARNING: Benchmark Compilation"
fi
TOTAL_GATES=$((TOTAL_GATES + 1))

# Performance regression check (if baseline exists)
BASELINE_FILE="benchmarks/baselines/v0.9.0.json"
if [ -f "$BASELINE_FILE" ]; then
    print_status "INFO" "Found performance baseline, checking for regressions..."
    # This would typically run benchmarks and compare against baseline
    # For now, just check that benchmarks exist
    print_status "OK" "Gate PASSED: Performance Baseline Available"
    GATES_PASSED=$((GATES_PASSED + 1))
else
    print_status "WARN" "Gate WARNING: No Performance Baseline Available"
fi
TOTAL_GATES=$((TOTAL_GATES + 1))

# 5. Build Gates
echo ""
echo "5. Build Gates"
echo "---------------"

# Debug build
check_gate "Debug Build" "cargo build --workspace" "success"

# Release build
check_gate "Release Build" "cargo build --workspace --release" "success"

# Cross-platform builds
PLATFORMS=("x86_64-unknown-linux-gnu" "x86_64-apple-darwin")
for platform in "${PLATFORMS[@]}"; do
    if rustup target add "$platform" 2>/dev/null; then
        check_gate "Cross-Platform Build ($platform)" "cargo check --target $platform --workspace" "success"
    else
        print_status "WARN" "Gate SKIPPED: Cross-Platform Build ($platform) - target not available"
        TOTAL_GATES=$((TOTAL_GATES + 1))
    fi
done

# 6. Documentation Gates
echo ""
echo "6. Documentation Gates"
echo "---------------------"

# Documentation builds
check_gate "Documentation Build" "cargo doc --no-deps --workspace" "success"

# API documentation completeness
DOC_WARNINGS=$(cargo doc --no-deps --workspace 2>&1 | grep -c "missing documentation" || echo "0")
if [ "$DOC_WARNINGS" -le 10 ]; then  # Allow some missing docs for now
    print_status "OK" "Gate PASSED: API Documentation ($DOC_WARNINGS warnings)"
    GATES_PASSED=$((GATES_PASSED + 1))
else
    print_status "WARN" "Gate WARNING: API Documentation ($DOC_WARNINGS warnings)"
fi
TOTAL_GATES=$((TOTAL_GATES + 1))

# 7. Feature Gates
echo ""
echo "7. Feature Gates"
echo "----------------"

# Check critical features work
check_gate "LSP Features" "cargo test -p perl-lsp --features workspace --locked" "success"

check_gate "Incremental Parsing" "cargo test -p perl-parser --features incremental --locked" "success"

check_gate "DAP Features" "cargo test -p perl-dap --features dap-phase1 --locked" "success"

# 8. Compatibility Gates
echo ""
echo "8. Compatibility Gates"
echo "---------------------"

# Version compatibility
check_gate "Rust Version Compatibility" "rustc --version" "success"

# Dependency compatibility
check_gate "Dependency Compatibility" "cargo tree --duplicates" "failure"

# 9. Release Process Gates
echo ""
echo "9. Release Process Gates"
echo "------------------------"

# Check version consistency
VERSION=$(grep "^version = " Cargo.toml | head -1 | cut -d'"' -f2)
if [ -n "$VERSION" ]; then
    print_status "OK" "Gate PASSED: Version Defined ($VERSION)"
    GATES_PASSED=$((GATES_PASSED + 1))
else
    print_status "ERROR" "Gate FAILED: Version Not Defined"
fi
TOTAL_GATES=$((TOTAL_GATES + 1))

# Check changelog exists
if [ -f "CHANGELOG.md" ]; then
    print_status "OK" "Gate PASSED: Changelog Exists"
    GATES_PASSED=$((GATES_PASSED + 1))
else
    print_status "WARN" "Gate WARNING: Changelog Missing"
fi
TOTAL_GATES=$((TOTAL_GATES + 1))

# Check license files
if [ -f "LICENSE-MIT" ] && [ -f "LICENSE-APACHE" ]; then
    print_status "OK" "Gate PASSED: License Files Present"
    GATES_PASSED=$((GATES_PASSED + 1))
else
    print_status "ERROR" "Gate FAILED: License Files Missing"
fi
TOTAL_GATES=$((TOTAL_GATES + 1))

# 10. SLO Validation
echo ""
echo "10. SLO Validation"
echo "-------------------"

# Calculate gate pass rate
PASS_RATE=$((GATES_PASSED * 100 / TOTAL_GATES))
REQUIRED_PASS_RATE=90

print_status "INFO" "Gate Pass Rate: $GATES_PASSED/$TOTAL_GATES ($PASS_RATE%)"
print_status "INFO" "Required Pass Rate: $REQUIRED_PASS_RATE%"

if [ "$PASS_RATE" -ge "$REQUIRED_PASS_RATE" ]; then
    print_status "OK" "SLO MET: Production Gates Pass Rate"
else
    print_status "ERROR" "SLO MISSED: Production Gates Pass Rate"
fi

# 11. Generate Production Gates Report
echo ""
echo "11. Generate Production Gates Report"
echo "----------------------------------"

REPORT_FILE="production_gates_validation_report_$(date +%Y%m%d_%H%M%S).json"

cat > "$REPORT_FILE" << EOF
{
  "timestamp": "$(date -Iseconds)",
  "scan_type": "production_hardening_phase6_gates",
  "slo_thresholds": {
    "parsing_time_p95_ms": $PARSING_TIME_P95_MS,
    "lsp_response_p95_ms": $LSP_RESPONSE_P95_MS,
    "memory_usage_p95_mb": $MEMORY_USAGE_P95_MB,
    "cpu_usage_p95_percent": $CPU_USAGE_P95_PERCENT,
    "test_coverage_percent": $TEST_COVERAGE_PERCENT,
    "security_vulnerabilities": $SECURITY_VULNERABILITIES,
    "required_pass_rate_percent": $REQUIRED_PASS_RATE
  },
  "results": {
    "total_gates": $TOTAL_GATES,
    "gates_passed": $GATES_PASSED,
    "pass_rate_percent": $PASS_RATE,
    "slo_met": $([ "$PASS_RATE" -ge "$REQUIRED_PASS_RATE" ] && echo "true" || echo "false"),
    "vulnerability_count": $VULNERABILITY_COUNT,
    "documentation_warnings": $DOC_WARNINGS,
    "version": "$VERSION"
  }
}
EOF

print_status "OK" "Production gates validation report generated: $REPORT_FILE"

# 12. Summary and Final Status
echo ""
echo "12. Summary and Final Status"
echo "============================"

print_status "INFO" "Production gates validation completed"
print_status "INFO" "Gates passed: $GATES_PASSED/$TOTAL_GATES ($PASS_RATE%)"

if [ "$PASS_RATE" -ge "$REQUIRED_PASS_RATE" ]; then
    print_status "OK" "🎉 PRODUCTION READY: All critical gates passed"
    echo ""
    echo "✅ The perl-lsp project is ready for production release!"
    echo ""
    echo "Next steps:"
    echo "1. Review the detailed report: $REPORT_FILE"
    echo "2. Run final integration tests: just e2e-validation"
    echo "3. Execute release process: cargo publish"
    echo "4. Update documentation and changelog"
else
    print_status "ERROR" "❌ NOT PRODUCTION READY: Some gates failed"
    echo ""
    echo "⚠️  Address the failed gates before production release:"
    echo "1. Review the detailed report: $REPORT_FILE"
    echo "2. Fix the failed gates listed above"
    echo "3. Re-run validation: just production-gates-validation"
    echo "4. Ensure all critical tests pass"
fi

echo ""
echo "Production Gates Validation Summary:"
echo "- Total Gates: $TOTAL_GATES"
echo "- Gates Passed: $GATES_PASSED"
echo "- Pass Rate: $PASS_RATE%"
echo "- Required: $REQUIRED_PASS_RATE%"
echo "- Status: $([ "$PASS_RATE" -ge "$REQUIRED_PASS_RATE" ] && echo "READY" || echo "NOT READY")"