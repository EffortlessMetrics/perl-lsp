#!/usr/bin/env bash
# Documentation hygiene checker
# Finds potential issues in doc comments that could cause rustdoc problems

set -euo pipefail

YELLOW='\033[1;33m'
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${YELLOW}=== Documentation Hygiene Check ===${NC}"
echo ""

# Track if we found any issues
found_issues=0

# 1. Check for unescaped brackets (potential broken links)
echo -e "${BLUE}Checking for unescaped brackets in doc comments...${NC}"
if rg -n --glob 'crates/**/src/**/*.rs' '^[ \t]*//[/!].*\[(?!(?:derive|test|cfg|allow|deny|warn|forbid|deprecated|must_use|inline|cold|export_name|link|link_name|no_mangle|repr|used|automatically_derived|no_std|no_implicit_prelude|should_panic|ignore|no_run|compile_fail)).*\]' 2>/dev/null; then
    echo -e "${YELLOW}⚠ Found potential unescaped brackets. Consider:${NC}"
    echo "  - Escaping with backslash: \\[text\\]"
    echo "  - Wrapping in code blocks: \`[text]\`"
    echo "  - Using proper doc links: [\`Type\`] or [Type](link)"
    found_issues=1
else
    echo -e "${GREEN}✓ No suspicious brackets found${NC}"
fi
echo ""

# 2. Check for bare URLs (should be wrapped in <>)
echo -e "${BLUE}Checking for bare URLs in doc comments...${NC}"
if rg -n --glob 'crates/**/src/**/*.rs' '^[ \t]*//[/!].*https?://[^ \t<>\[\]]+' 2>/dev/null | grep -v '<http'; then
    echo -e "${YELLOW}⚠ Found bare URLs. Wrap them in angle brackets: <https://example.com>${NC}"
    found_issues=1
else
    echo -e "${GREEN}✓ No bare URLs found${NC}"
fi
echo ""

# 3. Check for common doc comment issues
echo -e "${BLUE}Checking for other documentation issues...${NC}"

# Missing space after comment marker
if rg -n --glob 'crates/**/src/**/*.rs' '^[ \t]*//[/!][^ /!#\[]' 2>/dev/null | head -5; then
    echo -e "${YELLOW}⚠ Found doc comments without space after marker${NC}"
    echo "  Use: /// Text  or  //! Text"
    found_issues=1
fi

# Perl code examples without proper fencing
if rg -n --glob 'crates/**/src/**/*.rs' -A2 -B2 '^[ \t]*///.*\$[a-zA-Z_]' 2>/dev/null | grep -v '```' | grep '\$' | head -5; then
    echo -e "${YELLOW}⚠ Possible Perl code in docs without code blocks${NC}"
    echo "  Wrap Perl examples in triple backticks:"
    echo "  \`\`\`perl"
    echo "  my \$var = 42;"
    echo "  \`\`\`"
    found_issues=1
fi
echo ""

# 4. Check for unresolved markers in public docs
echo -e "${BLUE}Checking for TODOs in public documentation...${NC}"
if rg -n --glob 'crates/**/src/**/*.rs' '^[ \t]*///.*\b(TODO|FIXME|XXX|HACK)\b' 2>/dev/null; then
    echo -e "${YELLOW}⚠ Found TODO/FIXME in public docs (consider moving to regular comments)${NC}"
    found_issues=1
else
    echo -e "${GREEN}✓ No TODOs in public documentation${NC}"
fi
echo ""

# 5. Verify rustdoc builds cleanly
echo -e "${BLUE}Testing rustdoc build with strict flags...${NC}"
if RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls -D rustdoc::invalid_html_tags" cargo doc --workspace --no-deps >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Documentation builds cleanly${NC}"
else
    echo -e "${RED}✗ Documentation build failed with strict flags${NC}"
    echo "  Run to see errors:"
    echo "  RUSTDOCFLAGS=\"-D rustdoc::broken_intra_doc_links -D rustdoc::bare_urls\" cargo doc --workspace --no-deps"
    found_issues=1
fi
echo ""

# Summary
if [ $found_issues -eq 0 ]; then
    echo -e "${GREEN}=== All Documentation Checks Passed ===${NC}"
    exit 0
else
    echo -e "${YELLOW}=== Documentation Issues Found ===${NC}"
    echo "These are suggestions for improving documentation quality."
    echo "Not all issues are critical, but fixing them improves maintainability."
    exit 0  # Non-blocking by default
fi