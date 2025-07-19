#!/bin/bash
# Script to profile stack overflow in debug builds

set -e

YELLOW='\033[1;33m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}🔍 Profiling stack overflow in debug builds${NC}"
echo "=================================================="

cd "$(dirname "$0")/.."

# Ensure we're in debug mode
export CARGO_BUILD_MODE="debug"
export RUST_BACKTRACE=full

# Run the overflow test
echo -e "\n${YELLOW}Running deep nesting test...${NC}"
echo "This should trigger a stack overflow in debug builds"

# Run each test individually to capture different overflow patterns
for test in test_deep_nested_expression test_deep_nested_blocks test_deep_nested_arrays test_deep_method_chain; do
    echo -e "\n${YELLOW}Testing: $test${NC}"
    
    # Use timeout to prevent hanging
    if timeout 10s cargo test --features pure-rust --test debug_stack_overflow_test $test -- --ignored --nocapture 2>&1 | tee stack_trace_$test.log; then
        echo -e "${GREEN}✅ Test completed (unexpected - should overflow)${NC}"
    else
        exit_code=$?
        if [ $exit_code -eq 124 ]; then
            echo -e "${RED}⏱️ Test timed out after 10s${NC}"
        else
            echo -e "${RED}❌ Test failed with exit code: $exit_code${NC}"
        fi
        
        # Extract stack overflow info
        if grep -q "stack overflow" stack_trace_$test.log || grep -q "SIGSEGV" stack_trace_$test.log; then
            echo -e "${YELLOW}Stack overflow detected! Analyzing...${NC}"
            
            # Find the recursive pattern
            echo -e "\n${YELLOW}Recursive patterns found:${NC}"
            grep -E "(build_node|parse_|visit_)" stack_trace_$test.log | sort | uniq -c | sort -nr | head -20
        fi
    fi
done

echo -e "\n${YELLOW}📊 Summary${NC}"
echo "Stack traces saved to: stack_trace_*.log"
echo "Look for repeated function calls to identify recursion"