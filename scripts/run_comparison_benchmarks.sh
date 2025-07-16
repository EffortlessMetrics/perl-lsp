#!/bin/bash

# Tree-sitter Perl C vs Rust Benchmark Comparison Script
# This script runs benchmarks on both implementations and generates comparison results

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$PROJECT_ROOT/benchmark_results"
C_RESULTS="$RESULTS_DIR/c_implementation.json"
RUST_RESULTS="$RESULTS_DIR/rust_implementation.json"
COMPARISON_RESULTS="$RESULTS_DIR/comparison_results.json"
REPORT_FILE="$RESULTS_DIR/benchmark_report.md"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check dependencies
check_dependencies() {
    log_info "Checking dependencies..."
    
    local missing_deps=()
    
    if ! command -v cargo &> /dev/null; then
        missing_deps+=("cargo")
    fi
    
    if ! command -v node &> /dev/null; then
        missing_deps+=("node")
    fi
    
    if ! command -v jq &> /dev/null; then
        missing_deps+=("jq")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
        log_error "Please install the missing dependencies and try again."
        exit 1
    fi
    
    log_success "All dependencies found"
}

# Create results directory
setup_results_dir() {
    log_info "Setting up results directory..."
    mkdir -p "$RESULTS_DIR"
    log_success "Results directory created: $RESULTS_DIR"
}

# Run Rust benchmarks
run_rust_benchmarks() {
    log_info "Running Rust implementation benchmarks..."
    
    cd "$PROJECT_ROOT"
    
    if ! cargo xtask bench --save --output "$RUST_RESULTS"; then
        log_error "Failed to run Rust benchmarks"
        exit 1
    fi
    
    log_success "Rust benchmarks completed"
}

# Run C implementation benchmarks
run_c_benchmarks() {
    log_info "Running C implementation benchmarks..."
    
    cd "$PROJECT_ROOT"
    
    # Check if C implementation exists
    if [ ! -f "src/parser.c" ]; then
        log_error "C implementation not found. Please ensure the C implementation is available."
        exit 1
    fi
    
    # Run C benchmarks using Node.js
    if ! node scripts/run_c_benchmarks.js --output "$C_RESULTS"; then
        log_error "Failed to run C benchmarks"
        exit 1
    fi
    
    log_success "C benchmarks completed"
}

# Generate comparison results
generate_comparison() {
    log_info "Generating comparison results..."
    
    cd "$PROJECT_ROOT"
    
    # Use a Python script to generate comparison
    if ! python3 scripts/generate_comparison.py \
        --c-results "$C_RESULTS" \
        --rust-results "$RUST_RESULTS" \
        --output "$COMPARISON_RESULTS" \
        --report "$REPORT_FILE"; then
        log_error "Failed to generate comparison results"
        exit 1
    fi
    
    log_success "Comparison results generated"
}

# Validate results
validate_results() {
    log_info "Validating benchmark results..."
    
    if [ ! -f "$C_RESULTS" ]; then
        log_error "C benchmark results not found: $C_RESULTS"
        exit 1
    fi
    
    if [ ! -f "$RUST_RESULTS" ]; then
        log_error "Rust benchmark results not found: $RUST_RESULTS"
        exit 1
    fi
    
    if [ ! -f "$COMPARISON_RESULTS" ]; then
        log_error "Comparison results not found: $COMPARISON_RESULTS"
        exit 1
    fi
    
    # Check if results contain valid JSON
    if ! jq empty "$C_RESULTS" 2>/dev/null; then
        log_error "Invalid JSON in C results file"
        exit 1
    fi
    
    if ! jq empty "$RUST_RESULTS" 2>/dev/null; then
        log_error "Invalid JSON in Rust results file"
        exit 1
    fi
    
    if ! jq empty "$COMPARISON_RESULTS" 2>/dev/null; then
        log_error "Invalid JSON in comparison results file"
        exit 1
    fi
    
    log_success "All results validated"
}

# Display summary
display_summary() {
    log_info "Benchmark Summary"
    echo "=================="
    
    if [ -f "$COMPARISON_RESULTS" ]; then
        echo "Results saved to:"
        echo "  - C Implementation: $C_RESULTS"
        echo "  - Rust Implementation: $RUST_RESULTS"
        echo "  - Comparison Results: $COMPARISON_RESULTS"
        echo "  - Report: $REPORT_FILE"
        echo ""
        
        # Display key metrics
        if command -v jq &> /dev/null; then
            echo "Key Performance Metrics:"
            jq -r '.summary | to_entries[] | "  \(.key): \(.value)"' "$COMPARISON_RESULTS" 2>/dev/null || echo "  Unable to parse summary"
        fi
    fi
}

# Check performance gates
check_performance_gates() {
    log_info "Checking performance gates..."
    
    if [ ! -f "$COMPARISON_RESULTS" ]; then
        log_warning "No comparison results available for performance gate checking"
        return 0
    fi
    
    # Check if any tests exceed regression thresholds
    local regressions=0
    
    # Parse time regression check (5% threshold)
    local parse_regressions=$(jq -r '.tests[] | select(.parse_time_regression > 0.05) | .name' "$COMPARISON_RESULTS" 2>/dev/null | wc -l)
    regressions=$((regressions + parse_regressions))
    
    # Memory regression check (20% threshold)
    local memory_regressions=$(jq -r '.tests[] | select(.memory_regression > 0.20) | .name' "$COMPARISON_RESULTS" 2>/dev/null | wc -l)
    regressions=$((regressions + memory_regressions))
    
    if [ "$regressions" -gt 0 ]; then
        log_warning "Found $regressions performance regression(s)"
        return 1
    else
        log_success "All performance gates passed"
        return 0
    fi
}

# Main execution
main() {
    log_info "Starting Tree-sitter Perl C vs Rust benchmark comparison"
    echo "=========================================================="
    
    # Parse command line arguments
    local run_c=true
    local run_rust=true
    local validate_only=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --c-only)
                run_rust=false
                shift
                ;;
            --rust-only)
                run_c=false
                shift
                ;;
            --validate-only)
                validate_only=true
                shift
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --c-only        Run only C implementation benchmarks"
                echo "  --rust-only     Run only Rust implementation benchmarks"
                echo "  --validate-only Only validate existing results"
                echo "  --help          Show this help message"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Check dependencies
    check_dependencies
    
    # Setup results directory
    setup_results_dir
    
    if [ "$validate_only" = true ]; then
        validate_results
        check_performance_gates
        display_summary
        exit 0
    fi
    
    # Run benchmarks
    if [ "$run_rust" = true ]; then
        run_rust_benchmarks
    fi
    
    if [ "$run_c" = true ]; then
        run_c_benchmarks
    fi
    
    # Generate comparison
    if [ "$run_c" = true ] && [ "$run_rust" = true ]; then
        generate_comparison
    fi
    
    # Validate results
    validate_results
    
    # Check performance gates
    local gate_status=0
    check_performance_gates || gate_status=$?
    
    # Display summary
    display_summary
    
    # Exit with appropriate status
    if [ $gate_status -ne 0 ]; then
        log_warning "Performance gates failed - check results for regressions"
        exit $gate_status
    else
        log_success "Benchmark comparison completed successfully"
        exit 0
    fi
}

# Run main function
main "$@" 