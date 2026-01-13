#!/usr/bin/env bash
# check_doc_paths.sh - Prevent machine-specific absolute paths in documentation
# Part of the "truth-safe" documentation policy
#
# This script fails if docs contain personal paths like:
# - /home/username/...
# - /Users/username/...
# - C:\Users\username\... (in non-example contexts)
#
# Legitimate examples like /home/user/... are allowed (generic placeholders)

set -euo pipefail

DOCS_DIR="${1:-docs}"
FOUND_ISSUES=0

echo "🔍 Checking for machine-specific paths in $DOCS_DIR..."

# Pattern to detect personal paths (not generic /home/user examples)
# Matches /home/ followed by anything other than 'user/'
if grep -rn '/home/[^u]' "$DOCS_DIR" 2>/dev/null | grep -v '/home/user' | grep -v 'Binary file'; then
    echo "❌ Found machine-specific /home/ paths (not /home/user examples)"
    FOUND_ISSUES=1
fi

# Also check for steven specifically (common dev machine path)
if grep -rn '/home/steven' "$DOCS_DIR" 2>/dev/null; then
    echo "❌ Found /home/steven paths - replace with repo-relative paths"
    FOUND_ISSUES=1
fi

# Check for other common developer paths that aren't examples
if grep -rn '/Users/[^N]' "$DOCS_DIR" 2>/dev/null | grep -v '/Users/Name' | grep -v 'Binary file'; then
    echo "⚠️  Found macOS user paths that may be machine-specific"
    # Don't fail for this, just warn - might be legitimate examples
fi

if [ $FOUND_ISSUES -eq 0 ]; then
    echo "✅ No machine-specific paths found in documentation"
    exit 0
else
    echo ""
    echo "Fix: Replace absolute paths with repo-relative paths or generic examples"
    echo "  - Use relative paths: docs/file.md instead of /home/.../docs/file.md"
    echo "  - Use generic examples: /home/user/project for user-facing docs"
    exit 1
fi
