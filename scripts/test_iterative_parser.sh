#!/bin/bash
# Test script for iterative parser implementation

set -e

YELLOW='\033[1;33m'
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🧪 Testing Iterative Parser Implementation${NC}"
echo "============================================"

# Build the project
echo -e "\n${YELLOW}Building with pure-rust feature...${NC}"
cargo build --features pure-rust --quiet

# Run iterative parser tests
echo -e "\n${YELLOW}Running iterative parser tests...${NC}"
cargo test --features pure-rust iterative_parser_tests -- --nocapture

# Run benchmark comparison
echo -e "\n${YELLOW}Running parser benchmarks...${NC}"
cargo run --features pure-rust --bin benchmark_parsers

# Test deep nesting specifically
echo -e "\n${YELLOW}Testing deep nesting capabilities...${NC}"
cargo test --features pure-rust test_deep_nesting --nocapture

echo -e "\n${GREEN}✅ All iterative parser tests completed!${NC}"