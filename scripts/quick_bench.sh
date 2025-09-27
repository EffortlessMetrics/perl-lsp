#!/bin/bash

# Quick benchmark script to test a few files

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

cd /home/steven/code/tree-sitter-perl/crates/tree-sitter-perl-rs

echo -e "${BLUE}=== Quick Parser Comparison ===${NC}"
echo

# Test files
TEST_FILES=(
    "/home/steven/code/tree-sitter-perl/benchmark_tests/simple.pl"
    "/home/steven/code/tree-sitter-perl/benchmark_tests/medium.pl"
    "/home/steven/code/tree-sitter-perl/benchmark_tests/complex.pl"
    "/home/steven/code/tree-sitter-perl/benchmark_tests/edge_cases.pl"
    "/home/steven/code/tree-sitter-perl/benchmark_tests/fuzzed/stress_deep_nesting.pl"
    "/home/steven/code/tree-sitter-perl/benchmark_tests/fuzzed/stress_operators.pl"
)

echo "File,Size,C_Time(µs),Rust_Time(µs),Speedup"
echo "----,----,----------,-------------,-------"

for file in "${TEST_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        continue
    fi
    
    filename=$(basename "$file")
    filesize=$(stat -c%s "$file" 2>/dev/null || stat -f%z "$file" 2>/dev/null || echo "0")
    
    # Test C parser (capture all output)
    c_output=$(timeout 5s cargo run --quiet --release --features "c-scanner test-utils" --bin bench_parser -- "$file" 2>&1 || echo "FAILED")
    c_time=$(echo "$c_output" | grep "status=success" | grep -oE 'duration_us=[0-9]+' | sed 's/duration_us=//' | tail -1)
    if [ -z "$c_time" ]; then
        c_time="FAIL"
    fi
    
    # Test Rust parser (capture all output)
    rust_output=$(timeout 5s cargo run --quiet --release --features "pure-rust test-utils" --bin bench_parser -- "$file" 2>&1 || echo "FAILED")
    rust_time=$(echo "$rust_output" | grep "status=success" | grep -oE 'duration_us=[0-9]+' | sed 's/duration_us=//' | tail -1)
    if [ -z "$rust_time" ]; then
        rust_time="FAIL"
    fi
    
    # Calculate speedup
    if [[ "$c_time" != "FAIL" ]] && [[ "$rust_time" != "FAIL" ]]; then
        speedup=$(echo "scale=2; $c_time / $rust_time" | bc)
        if (( $(echo "$speedup > 1" | bc -l) )); then
            speedup_text="${speedup}x (Rust faster)"
        else
            speedup=$(echo "scale=2; $rust_time / $c_time" | bc)
            speedup_text="${speedup}x (C faster)"
        fi
    else
        speedup_text="N/A"
    fi
    
    printf "%-30s %8s %12s %12s %s\n" "$filename" "$filesize" "$c_time" "$rust_time" "$speedup_text"
done

echo
echo -e "${GREEN}Quick benchmark complete!${NC}"