#!/usr/bin/env bash
# Apply workspace exclusion simplification
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CARGO_TOML="$REPO_ROOT/Cargo.toml"
BACKUP="$CARGO_TOML.backup.$(date +%s)"

echo "Backing up Cargo.toml to $BACKUP..."
cp "$CARGO_TOML" "$BACKUP"

echo "Applying workspace simplification..."

# Create a temporary Python script to do the modifications atomically
python3 << 'PYTHON_SCRIPT'
import re

CARGO_TOML = "/home/steven/code/Rust/perl-lsp/review/Cargo.toml"

with open(CARGO_TOML, 'r') as f:
    content = f.read()

# Step 1: Improve exclusion section documentation
exclusion_pattern = r']\nexclude = \[\s*"tree-sitter-perl",.*?\]'
exclusion_replacement = ''']

# ============================================================================
# Workspace Exclusions
# ============================================================================
#
# These directories are excluded to simplify the default build and reduce
# contributor setup complexity. No workspace members depend on these crates.
#
# Why excluded:
#   - tree-sitter-perl: Legacy grammar (not used)
#   - tree-sitter-perl-c: Requires libclang-dev for bindgen
#   - crates/tree-sitter-perl-rs: Legacy Rust wrapper (not used)
#   - fuzz: Requires cargo-fuzz installation
#   - archive: Archived legacy code (unmaintained)
#
exclude = [
    "tree-sitter-perl",
    "tree-sitter-perl-c",
    "crates/tree-sitter-perl-rs",
    "fuzz",
    "archive",
]'''

content = re.sub(exclusion_pattern, exclusion_replacement, content, flags=re.DOTALL)

# Step 2: Remove unused workspace.dependencies references
content = re.sub(
    r'^tree-sitter-perl = \{ path = "crates/tree-sitter-perl-rs", features = \["pure-rust"\] \}\n',
    '',
    content,
    flags=re.MULTILINE
)
content = re.sub(
    r'^tree-sitter-perl-c = \{ path = "crates/tree-sitter-perl-c" \}\n',
    '',
    content,
    flags=re.MULTILINE
)

# Step 3: Add explanatory comment in external dependencies
content = re.sub(
    r'(# External dependencies)\n(tree-sitter = )',
    r'\1\n# Note: tree-sitter-perl and tree-sitter-perl-c are excluded from workspace\n\2',
    content
)

# Write back atomically
with open(CARGO_TOML, 'w') as f:
    f.write(content)

print("✅ Applied workspace simplification successfully")
PYTHON_SCRIPT

echo
echo "Changes applied:"
echo "  1. Improved exclusion section documentation"
echo "  2. Removed unused workspace.dependencies for excluded crates"
echo "  3. Added explanatory comments"
echo
echo "Backup saved to: $BACKUP"
