#!/bin/bash

# Comprehensive benchmark script for fuzzed test files
# Compares C/tree-sitter and pure Rust parser performance

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Output directory
RESULTS_DIR="/home/steven/code/tree-sitter-perl/benchmark_results/fuzzed_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$RESULTS_DIR"

echo -e "${BLUE}=== Tree-sitter Perl Parser Benchmark (Fuzzed Files) ===${NC}"
echo "Results will be saved to: $RESULTS_DIR"
echo

# Change to crates directory
cd /home/steven/code/tree-sitter-perl/crates/tree-sitter-perl-rs || exit 1

# Build both parsers (commented out - run manually if needed)
# echo -e "${YELLOW}Building parsers...${NC}"
# cargo build --release --features "c-scanner test-utils"
# cargo build --release --features "pure-rust test-utils"
echo -e "${YELLOW}Running benchmarks (parsers should be pre-built)...${NC}"

# Function to benchmark a parser
benchmark_parser() {
    local parser_type=$1
    local file_path=$2
    local output_file=$3
    
    if [ "$parser_type" = "c" ]; then
        # Run C parser benchmark
        timeout 5s cargo run --release --features "c-scanner test-utils" --bin bench_parser -- "$file_path" 2>&1
    else
        # Run Rust parser benchmark
        timeout 5s cargo run --release --features "pure-rust test-utils" --bin bench_parser -- "$file_path" 2>&1
    fi
}

# CSV header
echo "file,size_bytes,c_time_us,rust_time_us,c_status,rust_status,speedup" > "$RESULTS_DIR/results.csv"

# Summary statistics
total_files=0
c_success=0
rust_success=0
c_failures=0
rust_failures=0
c_total_time=0
rust_total_time=0

# Process all test files
BASE_DIR="/home/steven/code/tree-sitter-perl"
for category in "$BASE_DIR"/benchmark_tests/*.pl "$BASE_DIR"/benchmark_tests/fuzzed/*.pl; do
    [ -f "$category" ] || continue
    
    filename=$(basename "$category")
    filesize=$(stat -c%s "$category" 2>/dev/null || stat -f%z "$category" 2>/dev/null || echo "0")
    
    echo -n "Testing $filename... "
    
    # Test C parser
    c_output=$(benchmark_parser "c" "$category" "$RESULTS_DIR/c_$filename.out" 2>&1 || echo "FAILED")
    if [[ "$c_output" == *"FAILED"* ]] || [[ "$c_output" == *"Path does not exist"* ]]; then
        c_time="N/A"
        c_status="FAIL"
        ((c_failures++))
    else
        # Parse output format: status=success error=false duration_us=123
        c_time=$(echo "$c_output" | grep -oE 'duration_us=[0-9]+' | sed 's/duration_us=//' | head -1 || echo "N/A")
        if [[ "$c_time" != "N/A" ]] && [[ "$c_output" == *"status=success"* ]]; then
            c_status="OK"
            ((c_success++))
            c_total_time=$(echo "$c_total_time + $c_time" | bc)
        else
            c_time="N/A"
            c_status="FAIL"
            ((c_failures++))
        fi
    fi
    
    # Test Rust parser
    rust_output=$(benchmark_parser "rust" "$category" "$RESULTS_DIR/rust_$filename.out" 2>&1 || echo "FAILED")
    if [[ "$rust_output" == *"FAILED"* ]] || [[ "$rust_output" == *"Path does not exist"* ]]; then
        rust_time="N/A"
        rust_status="FAIL"
        ((rust_failures++))
    else
        # Parse output format: status=success error=false duration_us=123
        rust_time=$(echo "$rust_output" | grep -oE 'duration_us=[0-9]+' | sed 's/duration_us=//' | head -1 || echo "N/A")
        if [[ "$rust_time" != "N/A" ]] && [[ "$rust_output" == *"status=success"* ]]; then
            rust_status="OK"
            ((rust_success++))
            rust_total_time=$(echo "$rust_total_time + $rust_time" | bc)
        else
            rust_time="N/A"
            rust_status="FAIL"
            ((rust_failures++))
        fi
    fi
    
    # Calculate speedup
    if [[ "$c_time" != "N/A" ]] && [[ "$rust_time" != "N/A" ]]; then
        speedup=$(echo "scale=2; $c_time / $rust_time" | bc)
    else
        speedup="N/A"
    fi
    
    # Save to CSV
    echo "$filename,$filesize,$c_time,$rust_time,$c_status,$rust_status,$speedup" >> "$RESULTS_DIR/results.csv"
    
    # Display result
    if [[ "$c_status" == "OK" ]] && [[ "$rust_status" == "OK" ]]; then
        if [[ "$speedup" != "N/A" ]]; then
            if (( $(echo "$speedup > 1" | bc -l) )); then
                echo -e "${GREEN}✓${NC} C: ${c_time}µs, Rust: ${rust_time}µs (Rust ${speedup}x faster)"
            else
                echo -e "${GREEN}✓${NC} C: ${c_time}µs, Rust: ${rust_time}µs (C faster)"
            fi
        else
            echo -e "${GREEN}✓${NC} Both parsed successfully"
        fi
    elif [[ "$c_status" == "FAIL" ]] && [[ "$rust_status" == "OK" ]]; then
        echo -e "${YELLOW}⚠${NC} C parser failed, Rust OK (${rust_time}µs)"
    elif [[ "$c_status" == "OK" ]] && [[ "$rust_status" == "FAIL" ]]; then
        echo -e "${YELLOW}⚠${NC} Rust parser failed, C OK (${c_time}µs)"
    else
        echo -e "${RED}✗${NC} Both parsers failed"
    fi
    
    ((total_files++))
done

# Generate summary report
echo -e "\n${BLUE}=== Summary Report ===${NC}"
echo "Total files tested: $total_files"
echo
echo "C Parser:"
echo "  Success: $c_success ($((c_success * 100 / total_files))%)"
echo "  Failures: $c_failures"
if [[ $c_success -gt 0 ]]; then
    avg_c_time=$(echo "scale=2; $c_total_time / $c_success" | bc)
    echo "  Average time: ${avg_c_time}µs"
fi
echo
echo "Rust Parser:"
echo "  Success: $rust_success ($((rust_success * 100 / total_files))%)"
echo "  Failures: $rust_failures"
if [[ $rust_success -gt 0 ]]; then
    avg_rust_time=$(echo "scale=2; $rust_total_time / $rust_success" | bc)
    echo "  Average time: ${avg_rust_time}µs"
fi

# Performance comparison
if [[ $c_success -gt 0 ]] && [[ $rust_success -gt 0 ]]; then
    echo
    echo -e "${BLUE}Performance Comparison:${NC}"
    overall_speedup=$(echo "scale=2; $avg_c_time / $avg_rust_time" | bc)
    if (( $(echo "$overall_speedup > 1" | bc -l) )); then
        echo "  Rust parser is ${overall_speedup}x faster on average"
    else
        slowdown=$(echo "scale=2; $avg_rust_time / $avg_c_time" | bc)
        echo "  C parser is ${slowdown}x faster on average"
    fi
fi

# Save detailed report
cat > "$RESULTS_DIR/summary.txt" << EOF
Tree-sitter Perl Parser Benchmark Report
========================================
Date: $(date)
Total files tested: $total_files

C Parser Performance:
  Success rate: $c_success/$total_files ($((c_success * 100 / total_files))%)
  Failures: $c_failures
  Average parse time: ${avg_c_time:-N/A}µs
  Total time: ${c_total_time}µs

Rust Parser Performance:
  Success rate: $rust_success/$total_files ($((rust_success * 100 / total_files))%)
  Failures: $rust_failures
  Average parse time: ${avg_rust_time:-N/A}µs
  Total time: ${rust_total_time}µs

Performance Comparison:
  Overall speedup: ${overall_speedup:-N/A}x
  
Files with issues:
EOF

# List files where parsers disagree
grep -E "FAIL" "$RESULTS_DIR/results.csv" | while IFS=, read -r file size c_time rust_time c_status rust_status speedup; do
    if [[ "$c_status" != "$rust_status" ]]; then
        echo "  - $file: C=$c_status, Rust=$rust_status" >> "$RESULTS_DIR/summary.txt"
    fi
done

echo
echo -e "${GREEN}Full results saved to: $RESULTS_DIR${NC}"
echo "  - results.csv: Detailed results for each file"
echo "  - summary.txt: Overall summary report"