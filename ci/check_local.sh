#!/usr/bin/env bash
# Local quality check script - run before committing/pushing
# This mirrors CI checks to catch issues early

set -euo pipefail

YELLOW='\033[1;33m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Running Local Quality Checks ===${NC}"
echo ""

# 1. Format check
echo -e "${YELLOW}1. Format check...${NC}"
if cargo fmt --all -- --check; then
    echo -e "${GREEN}✓ Format check passed${NC}"
else
    echo -e "${RED}✗ Format check failed - run 'cargo fmt --all' to fix${NC}"
    exit 1
fi
echo ""

# 2. Clippy
echo -e "${YELLOW}2. Clippy analysis...${NC}"
if cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tee /tmp/clippy.log | grep -q "warning:"; then
    echo -e "${RED}✗ Clippy found warnings${NC}"
    cat /tmp/clippy.log
    exit 1
else
    echo -e "${GREEN}✓ Clippy check passed${NC}"
fi
echo ""

# 3. Documentation
echo -e "${YELLOW}3. Documentation build...${NC}"
if RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls" cargo doc --workspace --no-deps >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Documentation builds cleanly${NC}"
else
    echo -e "${RED}✗ Documentation build failed${NC}"
    echo "Run with full output: RUSTDOCFLAGS=\"-D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls\" cargo doc --workspace --no-deps"
    exit 1
fi
echo ""

# 4. Tests
echo -e "${YELLOW}4. Running tests...${NC}"
if cargo test --workspace --all-features --quiet; then
    echo -e "${GREEN}✓ All tests passed${NC}"
else
    echo -e "${RED}✗ Tests failed${NC}"
    exit 1
fi
echo ""

# 5. Ignored tests baseline
echo -e "${YELLOW}5. Checking ignored tests baseline...${NC}"
if [ -x "./ci/check_ignored.sh" ]; then
    if ./ci/check_ignored.sh; then
        echo -e "${GREEN}✓ Ignored tests baseline correct${NC}"
    else
        echo -e "${RED}✗ Ignored tests baseline mismatch${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠ Skipping ignored tests check (script not found)${NC}"
fi
echo ""

# 6. Optional: cargo deny (if available)
echo -e "${YELLOW}6. Dependency security check...${NC}"
if command -v cargo-deny &> /dev/null; then
    if cargo deny check 2>&1 | tee /tmp/deny.log | grep -q "error:"; then
        echo -e "${RED}✗ Dependency issues found${NC}"
        cat /tmp/deny.log
        exit 1
    else
        echo -e "${GREEN}✓ Dependencies are secure${NC}"
    fi
else
    echo -e "${YELLOW}⚠ cargo-deny not installed (run: cargo install cargo-deny)${NC}"
fi
echo ""

echo -e "${GREEN}=== All Local Checks Passed ===${NC}"
echo ""
echo "You can now safely commit/push your changes."
echo "Pro tip: Install as git pre-push hook: cp ci/check_local.sh .git/hooks/pre-push"