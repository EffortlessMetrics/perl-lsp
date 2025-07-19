#!/bin/bash
# Comprehensive test script for edge case handling

set -e

YELLOW='\033[1;33m'
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Testing Edge Case Handling ===${NC}"

# Run edge case specific tests
echo -e "\n${YELLOW}Running edge case tests...${NC}"
cargo test --features "pure-rust test-utils" edge_case_tests -- --nocapture

echo -e "\n${YELLOW}Running integration tests...${NC}"
cargo test --features "pure-rust test-utils" test_edge_case_integration -- --nocapture
cargo test --features "pure-rust test-utils" test_recovery_mode_effectiveness -- --nocapture
cargo test --features "pure-rust test-utils" test_encoding_aware_heredocs -- --nocapture

# Run benchmarks if requested
if [ "$1" == "--bench" ]; then
    echo -e "\n${YELLOW}Running edge case benchmarks...${NC}"
    cargo bench --features "pure-rust test-utils" edge_case_benchmarks
fi

# Run examples
echo -e "\n${YELLOW}Running edge case examples...${NC}"
cargo run --features "pure-rust test-utils" --example edge_case_demo
cargo run --features "pure-rust test-utils" --example anti_pattern_analysis
cargo run --features "pure-rust test-utils" --example tree_sitter_compatibility

echo -e "\n${GREEN}✓ All edge case tests passed!${NC}"

# Generate coverage report if requested
if [ "$1" == "--coverage" ]; then
    echo -e "\n${YELLOW}Generating coverage report...${NC}"
    cargo tarpaulin --features pure-rust --out Html --output-dir target/coverage
    echo -e "${GREEN}Coverage report generated at target/coverage/index.html${NC}"
fi