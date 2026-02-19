#!/usr/bin/env bash
# Validate workspace exclusion strategy
#
# This script validates that:
# 1. Excluded crates are properly documented
# 2. No workspace members depend on excluded crates
# 3. workspace.dependencies doesn't reference excluded crates

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CARGO_TOML="$REPO_ROOT/Cargo.toml"

echo "Validating workspace exclusion strategy..."
echo

# Check 1: Verify excluded crates exist
echo "✓ Checking excluded directories exist..."
for dir in "tree-sitter-perl" "crates/tree-sitter-perl-c" "fuzz" "archive"; do
    if [ ! -d "$REPO_ROOT/$dir" ]; then
        echo "❌ ERROR: Excluded directory '$dir' does not exist"
        exit 1
    fi
done
echo "  All excluded directories exist"
echo

# Check 2: Verify exclusion documentation
echo "✓ Checking exclusion documentation..."
if ! grep -q "Workspace Exclusions" "$CARGO_TOML"; then
    echo "❌ ERROR: Exclusion strategy not documented in Cargo.toml"
    exit 1
fi
echo "  Exclusion strategy is documented"
echo

# Check 3: Verify workspace.dependencies doesn't reference excluded crates
echo "✓ Checking workspace.dependencies..."
if grep -A 100 "^\[workspace\.dependencies\]" "$CARGO_TOML" | \
   grep -v "^#" | \
   grep -E "tree-sitter-perl\s*=|tree-sitter-perl-c\s*=" > /dev/null 2>&1; then
    echo "❌ ERROR: workspace.dependencies references excluded crates"
    exit 1
fi
echo "  workspace.dependencies clean (no excluded crate references)"
echo

# Check 4: Verify excluded crates are listed in exclude section
echo "✓ Checking exclude section..."
for crate in "tree-sitter-perl" "tree-sitter-perl-c" "fuzz" "archive"; do
    if ! grep -A 10 "^exclude = \[" "$CARGO_TOML" | grep -q "\"$crate\""; then
        echo "❌ ERROR: '$crate' not found in exclude section"
        exit 1
    fi
done
echo "  All expected crates are in exclude section"
echo

# Check 5: Verify workspace members don't include excluded crates
echo "✓ Checking workspace members..."
WORKSPACE_MEMBERS=$(cargo metadata --format-version=1 --no-deps | jq -r '.workspace_members[]')
if echo "$WORKSPACE_MEMBERS" | grep -q "tree-sitter-perl"; then
    echo "❌ ERROR: Excluded crates found in workspace members"
    exit 1
fi
MEMBER_COUNT=$(echo "$WORKSPACE_MEMBERS" | wc -l)
echo "  Workspace has $MEMBER_COUNT members (excluded crates not included)"
echo

# Check 6: Verify no workspace member Cargo.toml depends on excluded crates
echo "✓ Checking for dependencies on excluded crates..."
FOUND_DEPS=0
for cargo_toml in "$REPO_ROOT"/crates/*/Cargo.toml; do
    if [ -f "$cargo_toml" ]; then
        if grep -E "tree-sitter-perl\s*=|tree-sitter-perl-c\s*=" "$cargo_toml" > /dev/null 2>&1; then
            echo "❌ ERROR: $(basename $(dirname $cargo_toml)) depends on excluded crate"
            FOUND_DEPS=1
        fi
    fi
done
if [ $FOUND_DEPS -eq 1 ]; then
    exit 1
fi
echo "  No workspace members depend on excluded crates"
echo

echo "=========================================="
echo "✅ All workspace exclusion checks passed!"
echo "=========================================="
echo
echo "Summary:"
echo "  - 4 directories excluded from workspace"
echo "  - Exclusion strategy clearly documented"
echo "  - No accidental dependencies on excluded crates"
echo "  - workspace.dependencies clean"
echo
